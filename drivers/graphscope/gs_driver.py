import sys
import graphscope as gs
import psycopg
import psycopg.sql as sql
import yaml
import time
import pandas as pd
from graphscope.framework.graph import Graph, GraphDAGNode

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

# gremlin queries for number of vertexes and edges
def graph_vertex_edge_count(sess:gs.Session, g:Graph | GraphDAGNode):
    itr = sess.gremlin(g)
    gt = itr.traversal_source()
    tot_vertex = gt.V().count().toList()[0]
    tot_edges = gt.E().count().toList()[0]
    return tot_vertex, tot_edges


def log_metrics_sql(conn: psycopg.Connection, log_id:int, algo:str, dataset:str, type_:str, time:float, vertex:int, edge:int)->None:
    columns = ["id", "algo", "dataset", "type", "time", "vertex", "edge"]
    cur = conn.cursor()
    query = sql.SQL("INSERT INTO gn_test ({}) VALUES ({})").format(
            sql.SQL(', ').join(map(sql.Identifier, columns)),
            sql.SQL(', ').join(sql.Placeholder() * len(columns)))

    time_ms = time // 1000000
    cur.execute(query, (log_id, algo, dataset, type_, time_ms, vertex, edge))
    conn.commit()
    cur.close()

def load_data(config, sess:gs.Session, vertex_file:str, edge_file:str):
    """
    Returns loading time, loaded graph, vertex number, edge_number
    """
    #v = loader.Loader(f"file://{vertex_file}", header_row=False)
    #e = loader.Loader(f"file://{edge_file}", header_row=False)
    g = sess.g(directed=config["dataset"]["directed"])

    df_v = pd.read_csv(vertex_file, header=None, names=["vertex"])    
    if not config["dataset"]["weights"]:
        df_e = pd.read_csv(edge_file, header=None, names=["src","dst"], sep=" ")
    else:
        df_e = pd.read_csv(edge_file, header=None, names=["src", "dst", "weights"], sep=" ")

    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    g = g.add_vertices(df_v).add_edges(df_e)
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

def main():
    # functional arguments position for the program
    # config_file_path id1 id2 algorithm dataset log_file
    
    config_yml = sys.argv[1]

    with open(config_yml, 'r') as yml_file:
        config = yaml.safe_load(yml_file)

    #sql params
    id_ = int(config["config"]["id"])
    algo = config["config"]["algo"]

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
    
    #vertex = graph_vertex_count(g)
    #edge = graph_edge_count(g)

    log_metrics_sql(conn, id_, algo, dataset, "loading", duration, vertex, edge)

    func_d = {'bfs': bfs, 'pr':pr, 'wcc':wcc, 'cdlp':cdlp, 'lcc':lcc, 'sssp':sssp}

    dur = func_d[algo](config, g) 

    if dur > 0:
        log_metrics_sql(conn, id_, algo, dataset, "runtime", dur, vertex, edge)

    lf.close()
    sess.close()
    conn.close()

main()
