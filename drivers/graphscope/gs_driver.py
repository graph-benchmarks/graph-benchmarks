import sys
import graphscope as gs
import psycopg
import psycopg.sql as sql
import yaml
import time
from graphscope.framework import loader

# functional arguments position for the program
# config_file_path id1 id2 algorithm dataset

config_yml = sys.argv[1]
_id = int(sys.argv[2])
_id2 = int(sys.argv[3])
algo = sys.argv[4]
dataset = sys.argv[5]

with open(config_yml, 'r') as yml_file:
    configs = yaml.safe_load(yml_file)

gs_host = configs["graphscope"]["host"]
gs_port = configs["graphscope"]["port"]

try:
    sess = gs.session(addr=f"{gs_host}:{gs_port}")
except:
    print("Error: could not connect to graphscope cluster")
    exit()
g = sess.g()

pg_host = configs["postgres"]["host"]
pg_db = configs["postgres"]["db"]
pg_port = configs["postgres"]["port"]
user_ps = configs["postgres"]["ps"]
pg_user = configs["postgres"]["user"]

try:
    conn = psycopg.connect(f"postgresql://{pg_user}:{user_ps}@{pg_host}:{pg_port}/{pg_db}")
except:
    print("Error: could not connect to postgresql database")
    exit()

# check if table exists on postgres
def check_table()->None:
    cur = conn.cursor()
    query = sql.SQL("SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'gn_test') AS table_existence")
    ret = cur.execute(query)
    
    if not ret.fetchone()[0]:
        query = sql.SQL("CREATE TABLE gn_test(id INTEGER, algo VARCHAR(256), dataset VARCHAR(256), type VARCHAR(256), time INTEGER)")  
        cur.execute(query)
 
    conn.commit()       
    cur.close()

def log_metrics_sql(log_id:int, type_:str, time:float)->None:
    columns = ["id", "algo", "dataset", "type", "time"]
    cur = conn.cursor()
    query = sql.SQL("INSERT INTO gn_test ({}) VALUES ({})").format(
            sql.SQL(', ').join(map(sql.Identifier, columns)),
            sql.SQL(', ').join(sql.Placeholder() * len(columns)))

    cur.execute(query, (log_id, algo, dataset, type_, time))
    conn.commit()
    cur.close()

# load vertex, edge files
def load_data(g: gs.Graph): 
    vertex_file = configs["dataset"][dataset]["vertex"]
    edge_file = configs["dataset"][dataset]["edges"] 
    
    v = loader.Loader(f"file://{vertex_file}", header_row=False)
    e = loader.Loader(f"file://{edge_file}", header_row=False)

    start_time = time.time()
    g = g.add_vertices(v, 'vertex')
    g = g.add_edges(e, 'edges')
    end_time = time.time()

    duration = end_time - start_time
    log_metrics_sql(_id, "loading", duration)


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

check_table()
load_data(g)
func_d = {'bfs': bfs, 'pr':pr, 'wcc':wcc, 'cdlp':cdlp, 'lcc':lcc, 'sssp':sssp}

start_time = time.time()
func_d[algo](g)
end_time = time.time()

duration = end_time - start_time
log_metrics_sql(_id2, "runtime", duration)

sess.close()
conn.close()
