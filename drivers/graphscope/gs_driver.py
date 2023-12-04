import pandas as pd
import sys
import graphscope as gs
import psycopg
import psycopg.sql as sql
import yaml
import time
from graphscope.framework.graph import Graph, GraphDAGNode
from graphscope.nx.classes.function import number_of_edges, number_of_nodes

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

def graph_vertex_count(g: Graph | GraphDAGNode)->int:
    return number_of_nodes(g) 

def graph_edge_count(g:Graph | GraphDAGNode)->int:
    return number_of_edges(g)

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

def load_data(g: Graph | GraphDAGNode, vertex_file:str, edge_file:str):
    """
    Returns loading time, loaded graph, vertex number, edge_number
    """
    #v = loader.Loader(f"file://{vertex_file}", header_row=False)
    #e = loader.Loader(f"file://{edge_file}", header_row=False)

    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    df_v = pd.read_csv(vertex_file, header=None, names=["vertex"])
    df_e = pd.read_csv(edge_file, header=None, names=["src","dst"])
    g = g.add_vertices(df_v).add_edges(df_e)
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)

    duration = end_time - start_time
    return duration, g, len(df_v), len(df_e)
    

def bfs(g):
    gs.bfs(g)
 
def pr(g):
    # figure out what max round is?
    gs.pagerank(g)

# weakly connected components
def wcc(g):
    gs.wcc(g)

# community detection using label propagation
def cdlp(g):
    gs.lpa(g)

# local cluster coefficient
def lcc(g):
    gs.avg_clustering(g)

# single source shortest paths
def sssp(g):
    gs.sssp(g)

def main():
    # functional arguments position for the program
    # config_file_path id1 id2 algorithm dataset log_file
    
    config_yml = sys.argv[1]

    with open(config_yml, 'r') as yml_file:
        configs = yaml.safe_load(yml_file)

    #sql params
    id_ = int(configs["config"]["id"])
    algo = configs["config"]["algo"]

    log_file = configs["config"]["log_file"]
    lf = open(log_file, "w+")

    gs_host = configs["platform"]["host"]
    gs_port = configs["platform"]["port"]

    pg_host = configs["postgres"]["host"]
    pg_db = configs["postgres"]["db"]
    pg_port = configs["postgres"]["port"]
    user_ps = configs["postgres"]["ps"]
    pg_user = configs["postgres"]["user"]

    dataset = configs["dataset"]["name"]
    vertex_file = configs["dataset"]["vertex"]
    edge_file = configs["dataset"]["edges"] 
    
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
    
    g = sess.g()
    
    check_table(conn)
    [duration, g, vertex, edge] = load_data(g, vertex_file, edge_file)
    
    #vertex = graph_vertex_count(g)
    #edge = graph_edge_count(g)

    log_metrics_sql(conn, id_, algo, dataset, "loading", duration, vertex, edge)

    func_d = {'bfs': bfs, 'pr':pr, 'wcc':wcc, 'cdlp':cdlp, 'lcc':lcc, 'sssp':sssp}

    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    func_d[algo](g)
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)

    duration = end_time - start_time
    log_metrics_sql(conn, id_, algo, dataset, "runtime", duration, vertex, edge)

    lf.close()
    sess.close()
    conn.close()

main()
