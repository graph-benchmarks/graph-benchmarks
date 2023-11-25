import sys
import graphscope as gs
import psycopg
import yaml
import time

# functional arguments position for the program
# id1 id2 workers cpu memory algorithm dataset
config_yml = sys.argv[1]
id_ = int(sys.argv[2])
id2_ = int(sys.argv[3])
num_workers = int(sys.argv[4])
cpu = int(sys.argv[5])
mem_size = int(sys.argv[6])
algo = sys.argv[7]
dataset = sys.argv[8]

# requirements from yaml config file
# - path to graph files
# - connection to postgres
# - connection to graphscope
with open(config_yml, 'r') as yml_file:
    configs = yaml.safe_load(yml_file)

sess = gs.session(addr=configs["graphscope"]["ip"])
g = sess.g()

# connected to postgres database
conn = psycopg.connect(
    host=configs["postgres"]["ip"],
    database=configs["postgres"]["db"],
    user=configs["postgres"]["user"],
    password=configs["postgres"]["ps"])

def log_metrics_sql(log_id:int, type_:str, time:float)->None:
    cur = conn.cursor()
    sql = """INSERT INTO gn_test(ID, Algo, Dataset, CPU, Workers, MEM_SIZE, Type, Time) 
             VALUES(%s) RETURNING student_id;"""
    cur.execute(sql, (log_id, algo, dataset, cpu, num_workers, mem_size, type_, time))
    conn.commit()
    cur.close()

# load the files from volumes
def load_data():
    global g
    start_time = time.time()
    
    vertex_file = configs["dataset"][dataset]["vertex"]
    edge_file = configs["dataset"][dataset]["edges"]

    g = g.add_vertices(vertex_file, 'vertex')
    g = g.add_edges(edge_file, 'edges')

    end_time = time.time()
    duration = end_time - start_time
    log_metrics_sql(id_, "loading", duration)

def bfs():
    pg = gs.bfs(g)
 
def pr():
    # figure out what max round is?
    pg = gs.pagerank(g)

# weakly connected components
def wcc():
    pg = gs.wcc(g)

# community detection using label propagation
def cdlp():
    pg = gs.lpa(g)

# local cluster coefficient
def lcc():
    pg = gs.avg_clustering(g)

# single source shortest paths
def sssp():
    c = gs.sssp(g)

load_data()

if algo == 'bfs':
    start_time = time.time()
    bfs()
    end_time = time.time()
    duration = end_time - start_time
    log_metrics_sql(id2_, "runtime", duration)

elif algo == 'pr':
    start_time = time.time()
    pr()
    end_time = time.time()
    duration = end_time - start_time
    log_metrics_sql(id2_, "runtime", duration)

elif algo == 'wcc':
    start_time = time.time()
    wcc()
    end_time = time.time()
    duration = end_time - start_time
    log_metrics_sql(id2_, "runtime", duration)

elif algo == 'cdlp':
    start_time = time.time()
    cdlp()
    end_time = time.time()
    duration = end_time - start_time
    log_metrics_sql(id2_, "runtime", duration)

elif algo == 'lcc':
    start_time = time.time()
    lcc()
    end_time = time.time()
    duration = end_time - start_time
    log_metrics_sql(id2_, "runtime", duration)

else:
    start_time = time.time()
    sssp()
    end_time = time.time()
    duration = end_time - start_time
    log_metrics_sql(id2_, "runtime", duration)

sess.close()
conn.close()
