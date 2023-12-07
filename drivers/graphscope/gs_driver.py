from io import TextIOWrapper
import sys
import requests
import graphscope as gs
import psycopg
import psycopg.sql as sql
import yaml
import time
from graphscope.framework import loader
from graphscope.framework.graph import Graph, GraphDAGNode
from kubernetes import client, config as KubeConfig
import shutil

# check if table exists on postgres
def check_table(conn: psycopg.Connection)->None:
    cur = conn.cursor()
    query = sql.SQL("SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'gn_test') AS table_existence")
    ret = cur.execute(query)
    
    if not ret.fetchone()[0]:
        query = sql.SQL("CREATE TABLE gn_test(id INTEGER, algo VARCHAR(256), dataset VARCHAR(256), type VARCHAR(256), time INTEGER, vertex INTEGER, edge INTEGER, nodes INTEGER)")  
        cur.execute(query)
 
    conn.commit()       
    cur.close()

# gremlin queries for number of vertexes and edges
def graph_vertex_edge_count(sess:gs.Session, g:Graph | GraphDAGNode):
    itr = sess.gremlin(g)
    gt = itr.traversal_source()
    tot_vertex = gt.V().count().toList()[0]
    tot_edges = gt.E().count().toList()[0]
    return tot_vertex, tot_edges


def log_metrics_sql(conn: psycopg.Connection, log_id:int, algo:str, dataset:str, type_:str, time:float, vertex:int, edge:int, nodes: int)->None:
    columns = ["id", "algo", "dataset", "type", "time", "vertex", "edge", "nodes"]
    cur = conn.cursor()
    query = sql.SQL("INSERT INTO gn_test ({}) VALUES ({})").format(
            sql.SQL(', ').join(map(sql.Identifier, columns)),
            sql.SQL(', ').join(sql.Placeholder() * len(columns)))

    time_ms = time // 1000000
    cur.execute(query, (log_id, algo, dataset, type_, time_ms, vertex, edge, nodes))
    conn.commit()
    cur.close()

def graph_vertex_edge_count(sess:gs.Session, g:Graph | GraphDAGNode):
    itr = sess.interactive(g)
    gt = itr.traversal_source()
    tot_vertex = gt.V().count().toList()[0]
    tot_edges = gt.E().count().toList()[0]
    return tot_vertex, tot_edges

JOB_NAME="bench-graphscope-hdfs-loader"
def wait_for_job_completion(api_instance):
    job_completed = False
    while not job_completed:
        api_response = api_instance.read_namespaced_job_status(
            name=JOB_NAME,
            namespace="default")
        if api_response.status.succeeded is not None or \
                api_response.status.failed is not None:
            job_completed = True

def load_data(configs, sess:gs.Session, vertex_file:str, edge_file:str):
    """
    Returns loading time, loaded graph, vertex number, edge_number
    """

    shutil.copyfile('upload-files.sh', '/scratch/upload-files.sh')
    shutil.copyfile('core-site.xml', '/scratch/core-site.xml')

    KubeConfig.load_incluster_config()

    api = client.CoreV1Api()
    service = api.read_namespaced_service(name="nfs-service", namespace="default")
    nfs_ip = service.spec.cluster_ip

    container = client.V1Container(
        name=JOB_NAME,
        image="apache/hadoop:2.10",
        command=["bash", "/scratch/upload-files.sh", vertex_file, edge_file],
        env=[client.V1EnvVar("HADOOP_USER_NAME", "root")],
        volume_mounts=[
            client.V1VolumeMount(mount_path="/scratch",name="scratch"),
            client.V1VolumeMount(mount_path="/attached",name="bench-storage")
        ]
    )

    volumes = [
        client.V1Volume(name="scratch", nfs=client.V1NFSVolumeSource(path="/scratch", server=nfs_ip)),
        client.V1Volume(name="bench-storage", nfs=client.V1NFSVolumeSource(path="/bench-storage", server=nfs_ip))
    ]

    template = client.V1PodTemplateSpec(
        metadata=client.V1ObjectMeta(labels={"app": JOB_NAME}),
        spec=client.V1PodSpec(restart_policy="Never", containers=[container], volumes=volumes))

    spec = client.V1JobSpec(
        template=template,
        backoff_limit=0,
        ttl_seconds_after_finished=0)

    job = client.V1Job(
        api_version="batch/v1",
        kind="Job",
        metadata=client.V1ObjectMeta(name="bench-graphscope-hdfs-loader"),
        spec=spec)

    lf.write("starting copy job")
    api_response = client.BatchV1Api().create_namespaced_job(
        body=job,
        namespace="default")
    wait_for_job_completion(client.BatchV1Api())
    lf.write("done copying datasets to hdfs")

    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    v = loader.Loader(f"hdfs://hadoop-hadoop-hdfs-nn:9000{vertex_file}", header_row=False, delimiter=" ")
    e = loader.Loader(f"hdfs://hadoop-hadoop-hdfs-nn:9000{edge_file}", header_row=False, delimiter=" ")

    g = sess.g(directed=configs["dataset"]["directed"])
    g = g.add_vertices(v).add_edges(e)
    lf.write("done loading to graphscope")
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    duration = end_time - start_time

    [tot_vertex, tot_edges] = graph_vertex_edge_count(sess, g)
    return duration, g, tot_vertex, tot_edges
    

def bfs(config, g: Graph | GraphDAGNode)->int:
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    gs.bfs(g)
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time
 
def pr(config, g: Graph | GraphDAGNode )->int:
    # figure out what max round is?
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    gs.pagerank(g)
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time 

# weakly connected components
def wcc(config, g: Graph | GraphDAGNode)->int:
    start_time = 0
    end_time = 0
    if not config["dataset"]["directed"]:
        start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
        gs.wcc(g)
        end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time

# community detection using label propagation
def cdlp(config,g: Graph | GraphDAGNode)->int:
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    gs.lpa(g)
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time

# local cluster coefficient
def lcc(config, g: Graph | GraphDAGNode)->int:
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    gs.avg_clustering(g)
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time

# single source shortest paths
def sssp(config, g: Graph | GraphDAGNode)->int:
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)    
    if config["dataset"]["weights"]:
        gs.sssp(g, weight="weights")
    else:
        gs.sssp(g)
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time

lf: TextIOWrapper
def main():
    # functional arguments position for the program
    # config_file_path id1 id2 algorithm dataset log_file
    
    config_yml = sys.argv[1]

    with open(config_yml, 'r') as yml_file:
        config = yaml.safe_load(yml_file)

    #sql params
    ids = [int(x.strip()) for x in config["config"]["ids"].split(",")]
    algos = [x.strip() for x in config["config"]["algos"].split(",")]
    id_algos = list(zip(ids, algos))
    nodes = config["config"]["nodes"]

    log_file = config["config"]["log_file"]
    lf = open(log_file, "w+")

    gs_host = config["platform"]["host"]
    gs_port = config["platform"]["port"]

    pg_host = config["postgres"]["host"]
    pg_db = config["postgres"]["db"]
    pg_port = config["postgres"]["port"]
    user_ps = config["postgres"]["ps"]
    pg_user = config["postgres"]["user"]

    dataset = config["dataset"]["name"]
    vertex_file = config["dataset"]["vertex"]
    edge_file = config["dataset"]["edges"] 
    
    try:
        conn = psycopg.connect(f"postgresql://{pg_user}:{user_ps}@{pg_host}:{pg_port}/{pg_db}")
    except:
        lf.write("Error: could not connect to postgresql database\n")
        lf.close()
        quit(1)

    try:
        sess = gs.session(addr=f"{gs_host}:{gs_port}")
    except:
        lf.write("Error: could not connect to graphscope cluster\n")
        lf.close()
        conn.close()    
        quit(1)
    
    check_table(conn)
    [duration, g, vertex, edge] = load_data(config, sess, vertex_file, edge_file)

    entry: (int, str)
    for entry in id_algos:
        log_metrics_sql(conn, entry[0], entry[1], dataset, "loading", duration, vertex, edge, nodes)

    func_d = {'bfs': bfs, 'pr':pr, 'wcc':wcc, 'cdlp':cdlp, 'lcc':lcc, 'sssp':sssp}

    for entry in id_algos:
        lf.write("starting " + entry[1] + " with id " + str(entry[0]))
        requests.post('http://notifier:8080/starting')
        dur = func_d[entry[1]](config, g) 
        requests.post('http://notifier:8080/stopping')

        if dur > 0:
            log_metrics_sql(conn, entry[0], entry[1], dataset, "runtime", dur, vertex, edge, nodes)

    del g

    lf.close()
    sess.close()
    conn.close()

main()
