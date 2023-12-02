from io import TextIOWrapper
import sys
import traceback
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
        query = sql.SQL("CREATE TABLE gn_test(id INTEGER, algo VARCHAR(256), dataset VARCHAR(256), type VARCHAR(256), time INTEGER, vertex INTEGER, edge INTEGER)")  
        cur.execute(query)
 
    conn.commit()       
    cur.close()

def load_data(sess: DriverRemoteConnection, g: GraphTraversalSource, v:str, e:str, lf: TextIOWrapper)->int:
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC) 
    
    tx = g.tx()

    try:
        # add vertex to the graph line by line
        vf = open(v, "r")

        while True:
            vertex = vf.readline()
            if not vertex:
                break
            vertex = vertex.rstrip()
            gtx = tx.begin()
            gtx.add_v('vert').property('num', vertex).iterate()
            tx.commit()
                 
        ef = open(e, "r")
        while True:
            edge = ef.readline()
            if not edge:
                break
            [src, dest] = edge.split(",")
            dest = dest.rstrip() 

            srcV = g.V().has_label('vert').limit(1).next()
            desV = g.V().has_label('vert').limit(1).next()
                        
        vf.close()
        ef.close()
        
    except Exception:
        traceback.print_exc()
        lf.write("Error: could not load files\n")
        sess.close()
        lf.close()
        quit(1)

    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time
 
# load data using gremlin shell
def load_data_groovy(v: str, e:str, lf:TextIOWrapper)->int:
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)

    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time

def log_metrics_sql(conn: psycopg.Connection, log_id:int, algo:str, dataset:str, type_:str, time:float)->None:
    columns = ["id", "algo", "dataset", "type", "time"]
    cur = conn.cursor()
    query = sql.SQL("INSERT INTO gn_test ({}) VALUES ({})").format(
            sql.SQL(', ').join(map(sql.Identifier, columns)),
            sql.SQL(', ').join(sql.Placeholder() * len(columns)))

    cur.execute(query, (log_id, algo, dataset, type_, time))
    conn.commit()
    cur.close()    

def graph_vertex_count(g: GraphTraversalSource):
    c = g.V().count().toList()
    return c[0]

def graph_edge_count(g: GraphTraversalSource):
    c = g.E().count().toList()
    return c[0]

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
    
    with open(config_yml, 'r') as yml_file:
        configs = yaml.safe_load(yml_file)

    # sql params
    id_ = int(configs["config"]["id"])
    algo = configs["config"]["algo"]
       
    log_file = configs["config"]["log_file"]
    lf = open(log_file, "w+")

    janus_host = configs["platform"]["host"]
    janus_port = configs["platform"]["port"]

    pg_host = configs["postgres"]["host"]
    pg_db = configs["postgres"]["db"]
    pg_port = configs["postgres"]["port"]
    user_ps = configs["postgres"]["ps"]
    pg_user = configs["postgres"]["user"]

    vertex_file = configs["dataset"]["vertex"]
    edge_file = configs["dataset"]["edges"] 
    
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

    duration = load_data(sess, g, vertex_file, edge_file, lf)
    vertex_num = graph_vertex_count(g)
    edge_count = graph_edge_count(g)
    #log_metrics_sql(conn, id_, algo, dataset, "loading", duration)

    func_d = {'bfs': bfs, 'pr':pr, 'wcc':wcc, 'cdlp':cdlp, 'lcc':lcc, 'sssp':sssp}

    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    func_d[algo](g)
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)

    duration = end_time - start_time
    #log_metrics_sql(conn, id_, algo, dataset, "runtime", duration)

    lf.close()
    sess.close()
    conn.close()

main()
