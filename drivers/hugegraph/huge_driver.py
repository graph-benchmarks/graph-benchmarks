import sys
import psycopg
import psycopg.sql as sql
import yaml
import time
import requests

# check if table exists on postgres
def check_table(conn: psycopg.Connection)->None:
    cur = conn.cursor()
    query = sql.SQL("SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'gn_test') AS table_existence")
    ret = cur.execute(query)
    
    if not ret.fetchone()[0]:
        query = sql.SQL("CREATE TABLE gn_test(id INTEGER, algo VARCHAR(256), dataset VARCHAR(256), type VARCHAR(256), time INTEGER, vertex INTEGER, edge INTEGER)")  
    cur.execute(query)
    conn.commit()       
    cur.close()

def graph_vertex_count(url:str)->int:
    query = "hugegraph.traversal().V().count()"
    r = requests.get(f"{url}?gremlin={query}")
    return 0

def graph_edge_count(url:str)->int:
    query = "hugegraph.traversal().E().count()"
    r = requests.get(f"{url}?gremlin={query}")
    return 0

def log_metrics_sql(conn: psycopg.Connection, log_id:int, algo:str, dataset:str, type_:str, time:float, vertex:int, edge:int)->None:
    columns = ["id", "algo", "dataset", "type", "time", "vertex", "edge"]
    cur = conn.cursor()
    query = sql.SQL("INSERT INTO gn_test ({}) VALUES ({})").format(
            sql.SQL(', ').join(map(sql.Identifier, columns)),
            sql.SQL(', ').join(sql.Placeholder() * len(columns)))

    cur.execute(query, (log_id, algo, dataset, type_, time, vertex, edge))
    conn.commit()
    cur.close()    

def bfs(url:str):
    query = "hugegraph.traversal().V().repeat(out().simplePath().barrier()).until(__.not(out()))"
    r = requests.get(f"{url}?gremlin={query}") 

def pr(url:str):
    query = "hugegraph.traversal().V().pageRank().with_(PageRank.propertyName,'pageRank').values('pageRank')"
    r = requests.get(f"{url}?gremlin={query}")
    print(r.js)

# weakly connected components
def wcc(url:str):
    query = "hugegraph.traversal().V().connectedComponent().group().by('componentId')"
    r = requests.get(f"{url}?gremlin={query}")
    print(r)

# community detection using label propagation
def cdlp(url:str):
    query = "hugegraph.traversal().V('1:marko')"
    r = requests.get(f"{url}?gremlin={query}")

# local cluster coefficient
def lcc(url:str):
    query = "hugegraph.traversal().V('1:marko')"
    r = requests.get(f"{url}?gremlin={query}")

# single source shortest paths
def sssp(url:str):
    query = "hugegraph.traversal().V().shortesPath()"
    r = requests.get(f"{url}?gremlin={query}")

def main():
    # functional arguments position for the program
    # config_file_path id1 id2 algorithm dataset log_file
    
    config_yml = sys.argv[1]
    
    with open(config_yml, 'r') as yml_file:
        configs = yaml.safe_load(yml_file)
     
    id_ = int(configs["config"]["id"])
    algo = configs["config"]["algo"]

    log_file = configs["config"]["log_file"]
    lf = open(log_file, "w+")

    huge_host = configs["platform"]["host"]
    huge_port = configs["platform"]["port"]
    url = f"http://{huge_host}:{huge_port}/gremlin"

    pg_host = configs["postgres"]["host"]
    pg_db = configs["postgres"]["db"]
    pg_port = configs["postgres"]["port"]
    user_ps = configs["postgres"]["ps"]
    pg_user = configs["postgres"]["user"]

    vertex_file = configs["dataset"]["vertex"]
    edge_file = configs["dataset"]["edges"] 
    dataset = configs["dataset"]["name"]

    try:
        conn = psycopg.connect(f"postgresql://{pg_user}:{user_ps}@{pg_host}:{pg_port}/{pg_db}")
    except:
        lf.write("Error: could not connect to postgresql database\n")
        lf.close()
        quit(1)
    
    check_table(conn)
    #duration = load_data(g, vertex_file, edge_file)
    #log_metrics_sql(conn, load_id, algo, dataset, "loading", duration)
    
    vertex = graph_vertex_count(url)
    edge = graph_edge_count(url)

    func_d = {'bfs': bfs, 'pr':pr, 'wcc':wcc, 'cdlp':cdlp, 'lcc':lcc, 'sssp':sssp}

    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    func_d[algo](url)
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)

    duration = end_time - start_time
    duration = duration // 1000000
    print(id_, algo, dataset, "runtime", duration, vertex, edge)
    log_metrics_sql(conn, id_, algo, dataset, "runtime", duration, vertex, edge)

    lf.close()
    conn.close()

main()
