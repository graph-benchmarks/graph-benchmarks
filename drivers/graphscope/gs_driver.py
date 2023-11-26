import sys
import graphscope as gs
import psycopg
import yaml
import time

# functional arguments position for the program
# id1 id2 workers cpu memory algorithm dataset
config_yml = sys.argv[1]

# requirements from yaml config file
# - path to graph files
# - connection to postgres
# - connection to graphscope
with open(config_yml, 'r') as yml_file:
    configs = yaml.safe_load(yml_file)

dataset = configs["dataset"]
algo = configs["algo"]

sess = gs.session(addr=configs["ip"])
g = sess.g()

# connected to postgres database
conn = psycopg.connect(
    host=configs["postgres"]["ip"],
    database=configs["postgres"]["db"],
    user=configs["postgres"]["user"],
    password=configs["postgres"]["password"])

def log_metrics_sql(type_:str, time:float)->int:
    cur = conn.cursor()
    sql = """INSERT INTO gn_test(Algo, Dataset, Type, Time) 
             VALUES(%s) RETURNING ID;"""
    cur.execute(sql, (algo, dataset, type_, time))
    conn.commit()
    cur.close()
    return cur.fetchone()

# load the files from volumes
def load_data():
    global g
    start_time = time.time()
    
    vertex_file = configs["dataset"][dataset] + "_v"
    edge_file = configs["dataset"][dataset] + "_e"

    g = g.add_vertices(vertex_file, 'vertex')
    g = g.add_edges(edge_file, 'edges')

    end_time = time.time()
    duration = end_time - start_time
    log_metrics_sql("loading", duration)

algos = {
    "bfs": lambda: gs.bfs(g),
    "pr": lambda: gs.pagerank(g),
    "wcc": lambda: gs.wcc(g),
    "cdlp": lambda: gs.lpa(g),
    "lcc": lambda: gs.avg_clustering(g),
    "sssp": lambda: gs.sssp(g),
}

load_data()
start_time = time.time()
algos[algo]()
end_time = time.time()
duration = end_time - start_time
id = log_metrics_sql("runtime", duration)
with open(configs["output_path"], 'w') as file:
    yaml.dump({ "id": id }, file)

sess.close()
conn.close()
