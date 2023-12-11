import requests
import sys
import psycopg
import psycopg.sql as sql
import yaml
import time
import os
import graphdatascience as GDS
from neo4j import GraphDatabase, Session as NeoSession
from kubernetes import client, config as KubeConfig
from kubernetes.stream import stream


# check if table exists on postgres
def check_table(conn: psycopg.Connection) -> None:
    cur = conn.cursor()
    query = sql.SQL(
        "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'gn_test') AS table_existence"
    )
    ret = cur.execute(query)

    if not ret.fetchone()[0]:
        query = sql.SQL(
            "CREATE TABLE gn_test(id INTEGER, algo VARCHAR(256), dataset VARCHAR(256), type VARCHAR(256), time INTEGER, vertex INTEGER, edge INTEGER, nodes INTEGER)"
        )
        cur.execute(query)

    conn.commit()
    cur.close()


def log_metrics_sql(
    conn: psycopg.Connection,
    log_id: int,
    algo: str,
    dataset: str,
    type_: str,
    time: float,
    vertex: int,
    edge: int,
    nodes: int,
) -> None:
    columns = ["id", "algo", "dataset", "type", "time", "vertex", "edge", "nodes"]
    cur = conn.cursor()
    query = sql.SQL("INSERT INTO gn_test ({}) VALUES ({})").format(
        sql.SQL(", ").join(map(sql.Identifier, columns)),
        sql.SQL(", ").join(sql.Placeholder() * len(columns)),
    )

    time_ms = time // 1000000
    cur.execute(query, (log_id, algo, dataset, type_, time_ms, vertex, edge, nodes))
    conn.commit()
    cur.close()

def wait_for_neo_stateful_set_ready(api_instance, idx):
    ready = False
    while not ready:
        api_response = api_instance.read_namespaced_stateful_set_status(
            name=f"server-{idx}", namespace="default"
        )
        if api_response.status.available_replicas == api_response.status.replicas:
            ready = True

def wait_for_pod(api_instance, pod_name):
    while True:
        resp = stream(
            api_instance.connect_get_namespaced_pod_exec,
            pod_name,
            "default",
            command=["ls"],
            stderr=True,
            stdin=True,
            stdout=True,
            tty=True,
            _preload_content=False,
        )
            
        if not resp.is_open():
            time.sleep(1)
            continue

        data = ""
        while resp.is_open():
            resp.update(timeout=1)
            if resp.peek_stdout():
                data += resp.read_stdout()
            if resp.peek_stderr():
                data += resp.read_stderr()
        resp.close()
        if len(data) != 0:
            break

def wait_for_db_ready(session: NeoSession):
    while True:
        time.sleep(1)
        dbs = session.run("SHOW DATABASES")
        currCount = 0
        for db in dbs:
            if db["name"] == "neo4j" and db["currentStatus"] != "online":
                currCount += 1
        if currCount == 0:
            break

def load_data(gds: GDS.GraphDataScience, config, vertex_file: str, edge_file: str):
    if bool(config["load_data"]):
        neo = connect_neo4j(config)
        session = neo.session()
        session.run(f"DROP DATABASE neo4j")

        vertex_file_name = os.path.basename(vertex_file)
        edge_file_name = os.path.basename(edge_file)

        os.rename(f"{vertex_file}", f"/attached/import/{vertex_file_name}")
        os.rename(f"{edge_file}", f"/attached/import/{edge_file_name}")

        with open(f"/attached/import/{edge_file_name}", "r") as f:
            data = f.readline()
            add_weights = len(data.split(" ")) == 3

        with open("/attached/import/v_headers.v", "w+") as f:
            f.write("vertex:ID(vertex)\n")
            f.close()

        with open("/attached/import/e_headers.e", "w+") as f:
            f.write(":START_ID(vertex) :END_ID(vertex)")
            if add_weights:
                f.write(" weight:float")
            f.write("\n")
            f.close()

        status = os.system("helm repo add neo4j https://helm.neo4j.com/neo4j")
        if os.WEXITSTATUS(status) != 0:
            sys.exit(-1)

        num_instances = int(config["platform"]["neo_instances"])
        KubeConfig.load_incluster_config()
        api = client.CoreV1Api()
        for i in range(1, num_instances + 1):
            wait_for_pod(api, f"server-{i}-0")

        start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
        api_instance = client.CoreV1Api()
        resp = stream(
            api_instance.connect_get_namespaced_pod_exec,
            "server-1-0",
            "default",
            command=[
                "neo4j-admin",
                "database",
                "import",
                "full",
                f"--nodes=NODE=/import/v_headers.v,/import/{vertex_file_name}",
                f"--relationships=EDGE=/import/e_headers.e,/import/{edge_file_name}",
                "--delimiter=U+0020",
                "--trim-strings=true",
                "--overwrite-destination=true",
                "--expand-commands",
            ],
            stderr=True,
            stdin=True,
            stdout=True,
            tty=True,
            _preload_content=False,
        )

        if not resp.is_open():
            exit(-1)

        while resp.is_open():
            resp.update(timeout=1)
            if resp.peek_stdout():
                print("%s" % resp.read_stdout())
            if resp.peek_stderr():
                print("%s" % resp.read_stderr())
        resp.close()
        end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)

        api = client.CoreV1Api()
        for i in range(1, num_instances + 1):
            wait_for_pod(api, f"server-{i}-0")
            print(f"server {i} running")

        api = client.AppsV1Api()
        for i in range(1, num_instances + 1):
            wait_for_neo_stateful_set_ready(api, i)
            print(f"server {i} stateful set running")

        neo = connect_neo4j(config)
        session = neo.session()

        servers = session.run("SHOW SERVERS")
        for s in servers:
            if s["address"].startswith("server-1"):
                server_id = s["name"]
                break

        session.run(f"CREATE DATABASE neo4j OPTIONS {{existingData: 'use', existingDataSeedInstance: '{server_id}'}}")
        wait_for_db_ready(session)

        retry = True
        while retry:
            try:
                gds = connect_gds(config)
                retry = False
            except:
                time.sleep(1)
                print("retrying")
        gds.run_cypher("DROP INDEX node_index IF EXISTS")
        gds.run_cypher("CREATE INDEX node_index FOR (n:NODE) ON (n.vertex)")

    # import names according to projection
    if bool(config["load_data"]):
        if config["dataset"]["weights"]:
            G = gds.graph.project(
                "my-graph", ["NODE"], {"EDGE": {"properties": ["weight"]}}
            )
        else:
            G = gds.graph.project("my-graph", ["NODE"], "EDGE")
    else:
        G = gds.graph.get("my-graph")

    tot_vertex = int(gds.run_cypher(
        """MATCH (n:NODE)
                                RETURN count(n) as total"""
    ).iloc[0, 0])
    tot_edges = int(gds.run_cypher(
        """MATCH ()-[r:EDGE]->()
                               RETURN count(r) as total"""
    ).iloc[0, 0])

    os.rename(f"/attached/import/{vertex_file_name}", f"{vertex_file}")
    os.rename(f"/attached/import/{edge_file_name}", f"{edge_file}")

    return gds, G, end_time - start_time, tot_vertex, tot_edges


def bfs(config, gds: GDS.GraphDataScience, G) -> int:
    source_id = gds.find_node_id(["NODE"], {"vertex": "1"})
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    gds.bfs.stats(G, sourceNode=source_id)
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time


def pr(config, gds: GDS.GraphDataScience, G) -> int:
    # figure out what max round is?
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    gds.pageRank.stats(G, maxIterations=20)
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time


# weakly connected components
def wcc(config, gds: GDS.GraphDataScience, G) -> int:
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    gds.wcc.stats(G)
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time


# community detection using label propagation
def cdlp(config, gds: GDS.GraphDataScience, G) -> int:
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    gds.labelPropagation.stats(G, maxIterations=10)
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time


# local cluster coefficient
def lcc(config, gds: GDS.GraphDataScience, G) -> int:
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    gds.localClusteringCoefficient.stats(G)
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time


# single source shortest paths
def sssp(config, gds: GDS.GraphDataScience, G) -> int:
    source_id = gds.find_node_id(["NODE"], {"vertex": "1"})
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    if config["dataset"]["weights"]:
        gds.allShortestPaths.dijkstra.stats(
            G, sourceNode=source_id, relationshipWeightProperty="weight"
        )
    else:
        gds.allShortestPaths.dijkstra.stats(G, sourceNode=source_id)
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time

def connect_neo4j(config):
    neo_host = config["platform"]["host"]
    neo_port = config["platform"]["port"]
    neo_user = config["platform"]["user"]
    neo_password = config["platform"]["password"]
    return GraphDatabase.driver(
        f"neo4j://{neo_host}:{neo_port}", auth=(neo_user, neo_password), database="system"
    )

def connect_gds(config):
    neo_host = config["platform"]["host"]
    neo_port = config["platform"]["port"]
    neo_user = config["platform"]["user"]
    neo_password = config["platform"]["password"]
    return GDS.GraphDataScience(
        f"neo4j://{neo_host}:{neo_port}", auth=(neo_user, neo_password)
    )

def main():
    # functional arguments position for the program
    # config_file_path id1 id2 algorithm dataset log_file

    config_yml = sys.argv[1]

    with open(config_yml, "r") as yml_file:
        config = yaml.safe_load(yml_file)

    # sql params
    ids = [int(x.strip()) for x in config["config"]["ids"].split(",")]
    algos = [x.strip() for x in config["config"]["algos"].split(",")]
    id_algos = list(zip(ids, algos))
    nodes = int(config["config"]["nodes"])

    log_file = config["config"]["log_file"]
    lf = open(log_file, "w+")

    pg_host = config["postgres"]["host"]
    pg_db = config["postgres"]["db"]
    pg_port = config["postgres"]["port"]
    user_ps = config["postgres"]["ps"]
    pg_user = config["postgres"]["user"]

    dataset = config["dataset"]["name"]
    vertex_file = config["dataset"]["vertex"]
    edge_file = config["dataset"]["edges"]

    try:
        conn = psycopg.connect(
            f"postgresql://{pg_user}:{user_ps}@{pg_host}:{pg_port}/{pg_db}"
        )
    except:
        lf.write("Error: could not connect to postgresql database\n")
        lf.close()
        quit(1)

    try:
        gds = connect_gds(config)
    except:
        lf.write("Error: could not connect to neo4j cluster\n")
        lf.close()
        conn.close()
        quit(1)


    check_table(conn)

    [gds, G, duration, vertex, edge] = load_data(gds, config, vertex_file, edge_file)

    if bool(config["load_data"]):
        for entry in id_algos:
            log_metrics_sql(
                conn, entry[0], entry[1], dataset, "loading", duration, vertex, edge, nodes
            )

    func_d = {"bfs": bfs, "pr": pr, "wcc": wcc, "cdlp": cdlp, "lcc": lcc, "sssp": sssp}

    for entry in id_algos:
        id_ = entry[0]
        algo = entry[1]

        requests.post("http://notifier:8080/starting")
        dur = func_d[algo](config, gds, G)
        requests.post("http://notifier:8080/stopping")

        if dur > 0:
            log_metrics_sql(
                conn, id_, algo, dataset, "runtime", dur, vertex, edge, nodes
            )

    lf.close()
    gds.close()
    conn.close()


main()