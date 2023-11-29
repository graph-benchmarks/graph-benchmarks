import sys
from gremlin_python.process.traversal import PageRank
import psycopg
import psycopg.sql as sql
import yaml
import time
from gremlin_python import statics
from gremlin_python.structure.graph import Graph
from gremlin_python.process.graph_traversal import __, out
from gremlin_python.driver.driver_remote_connection import DriverRemoteConnection
from gremlin_python.process.anonymous_traversal import traversal, GraphTraversalSource

# check if table exists on postgres
def check_table(conn: psycopg.Connection)->None:
    cur = conn.cursor()
    query = sql.SQL("SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'gn_test') AS table_existence")
    ret = cur.execute(query)
    
    if not ret.fetchone()[0]:
        query = sql.SQL("CREATE TABLE gn_test(id INTEGER, algo VARCHAR(256), dataset VARCHAR(256), type VARCHAR(256), time INTEGER)")  
        cur.execute(query)
 
    conn.commit()       
    cur.close()

def log_metrics_sql(conn: psycopg.Connection, log_id:int, algo:str, dataset:str, type_:str, time:float)->None:
    columns = ["id", "algo", "dataset", "type", "time"]
    cur = conn.cursor()
    query = sql.SQL("INSERT INTO gn_test ({}) VALUES ({})").format(
            sql.SQL(', ').join(map(sql.Identifier, columns)),
            sql.SQL(', ').join(sql.Placeholder() * len(columns)))

    cur.execute(query, (log_id, algo, dataset, type_, time))
    conn.commit()
    cur.close()    

def bfs(g: GraphTraversalSource):
    g.V().repeat(out().simplePath().barrier()).until(__.not_(out()))

def pr(g: GraphTraversalSource):
    # figure out what max round is?
    g.V().pageRank().with_(PageRank.propertyName, 'pageRank').values('pageRank')

# weakly connected components
def wcc(g: GraphTraversalSource):
    g.V().connectedComponent().group().by('componentId')

# community detection using label propagation
def cdlp(g: GraphTraversalSource):
    g.V()

# local cluster coefficient
def lcc(g: GraphTraversalSource):
    g.V()

# single source shortest paths
def sssp(g: GraphTraversalSource):
    g.V().shortestPath()

def main():
    # functional arguments position for the program
    # config_file_path id1 id2 algorithm dataset log_file
    
    config_yml = sys.argv[1]
    load_id = int(sys.argv[2])
    compute_id = int(sys.argv[3])
    algo = sys.argv[4]
    dataset = sys.argv[5]
    log_file = sys.argv[6]

    lf = open(log_file, "w+")

    with open(config_yml, 'r') as yml_file:
        configs = yaml.safe_load(yml_file)

    janus_host = configs["platform"]["host"]
    janus_port = configs["platform"]["port"]

    pg_host = configs["postgres"]["host"]
    pg_db = configs["postgres"]["db"]
    pg_port = configs["postgres"]["port"]
    user_ps = configs["postgres"]["ps"]
    pg_user = configs["postgres"]["user"]

    vertex_file = configs["dataset"][dataset]["vertex"]
    edge_file = configs["dataset"][dataset]["edges"] 
    
    try:
        conn = psycopg.connect(f"postgresql://{pg_user}:{user_ps}@{pg_host}:{pg_port}/{pg_db}")
    except:
        lf.write("Error: could not connect to postgresql database\n")
        lf.close()
        quit(1)

    try:
        sess = DriverRemoteConnection(f"ws://{janus_host}:{janus_port}/gremlin", 'g')
    except:
        lf.write("Error: could not connect to graphscope cluster\n")
        lf.close()
        conn.close()    
        quit(1)
    
    g = traversal().withRemote(sess)
    check_table(conn) 
    #duration = load_data(g, vertex_file, edge_file)
    #log_metrics_sql(conn, load_id, algo, dataset, "loading", duration)

    func_d = {'bfs': bfs, 'pr':pr, 'wcc':wcc, 'cdlp':cdlp, 'lcc':lcc, 'sssp':sssp}

    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    func_d[algo](g)
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)

    duration = end_time - start_time
    log_metrics_sql(conn, compute_id, algo, dataset, "runtime", duration)

    lf.close()
    sess.close()
    conn.close()

main()
