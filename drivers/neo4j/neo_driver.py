import requests
import sys
import psycopg
import psycopg.sql as sql
import yaml
import time
import graphdatascience as GDS
import pandas as pd 

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

def log_metrics_sql(conn: psycopg.Connection, log_id:int, algo:str, dataset:str, type_:str, time:float, vertex:int, edge:int, nodes:int)->None:
    columns = ["id", "algo", "dataset", "type", "time", "vertex", "edge", "nodes"]
    cur = conn.cursor()
    query = sql.SQL("INSERT INTO gn_test ({}) VALUES ({})").format(
            sql.SQL(', ').join(map(sql.Identifier, columns)),
            sql.SQL(', ').join(sql.Placeholder() * len(columns)))

    time_ms = time // 1000000
    cur.execute(query, (log_id, algo, dataset, type_, time_ms, vertex, edge, nodes))
    conn.commit()
    cur.close()

def full_load(gds : GDS.GraphDataScience, queries, params):
    df_v: pd.DataFrame = params[0]
    df_e: pd.DataFrame = params[0]
    add_nodes_query = queries[0]
    add_relations_query = queries[1]
    
    rows = df_v.to_dict("records")
    tot_vertex = gds.run_cypher(add_nodes_query, params={"rows":rows}).iloc[0,0]
    
    rows =df_e.to_dict("records")
    tot_edges = gds.run_cypher(add_relations_query, params={"rows":rows}).iloc[0,0]

    return tot_vertex, tot_edges

def batch_load(gds: GDS.GraphDataScience, queries, params):
    batch_size = 5000 
    
    ve_count = [0, 0]
    for i in range(len(params)):
        dft:pd.DataFrame = params[i]
        query:str = queries[i]
        count = 0
        itr = len(dft) // batch_size + 1
        
        for j in range(itr):
            start_index = j * batch_size
            end_index = min(j * batch_size + batch_size, len(dft))
            df_load: pd.DataFrame = dft.iloc[start_index:end_index, :]
            rows = df_load.to_dict("records")
            count += gds.run_cypher(query, params={"rows":rows}).iloc[0,0]
        
        ve_count[i] = count
    return ve_count

def csv_load(gds: GDS.GraphDataScience, queries, params):
    # for this to work file has to be a csv with no header
    add_nodes_query = queries[0]
    add_relations_query = queries[1]

    tot_vertex = gds.run_cypher(add_nodes_query).iloc[0,0]
    tot_edges = gds.run_cypher(add_relations_query).iloc[0,0]

    return tot_vertex, tot_edges

def load_data(gds: GDS.GraphDataScience, config, vertex_file:str, edge_file:str, ltype: int):
    """
    0 = full_load
    1 = batch_load
    2 = csv_load
    Returns graph, loading time, vertex number, edge_number
    """
    #v = loader.Loader(f"file://{vertex_file}", header_row=False)
    #e = loader.Loader(f"file://{edge_file}", header_row=False)
    
    df_v = None
    df_e = None

    if not ltype == 2:
        df_v = pd.read_csv(vertex_file, header=None, names=["vertex"]) 

        if not config["dataset"]["weights"]:
            df_e = pd.read_csv(edge_file, header=None, names=["src","dst"], sep=" ")
        else:
            df_e = pd.read_csv(edge_file, header=None, names=["src", "dst", "weight"], sep=" ")
    
    if ltype == 2:
        add_nodes_query =  """LOAD CSV FROM 'file://{}' AS line
        CREATE (:node {{nid: line[0]}})
        RETURN count(*) as total""".format(vertex_file)

        d_str = "-" if not config["dataset"]["directed"] else "->" 
        w_str = '{weight: line[2]}' if config["dataset"]["weights"] else "" 
        
        add_relations_query = """LOAD CSV FROM 'file://{}' AS line
        UNWIND line[0] as nodeID
        UNWIND line[1] as destID
        MATCH (s:node {{nid: nodeID}})
        MATCH (d:node {{nid: destID}})
        MERGE (s)-[:EDGE{}]{}(d)
        RETURN count(*) as total""".format(edge_file, d_str, w_str)
        queries = (add_nodes_query, add_relations_query)
    
    else:
        add_nodes_query = """UNWIND $rows AS row
        MERGE (:node {nid: row.vertex})
        RETURN count(*) as total
        """
        d_str = "-" if not config["dataset"]["directed"] else "->" 
        w_str = '{weight: row.weight}' if config["dataset"]["weights"] else ""        
        
        add_relations_query = """UNWIND $rows AS row
        UNWIND row.src AS nodeID
        UNWIND row.dst AS destID
        MATCH (s:node {{nid: nodeID}})
        MATCH (d:node {{nid: destID}})
        MERGE (s)-[:EDGE {}]{}(d)
        RETURN count(*) as total
        """.format(w_str, d_str)
        queries = (add_nodes_query, add_relations_query)

    load_lst = [full_load, batch_load, csv_load]
    
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    [tot_vertex, tot_edges] = load_lst[ltype](gds, queries, (df_v, df_e)) 
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    
    if not config["dataset"]["directed"]:
        G, result = gds.graph.project("my-graph", ["node"], "EDGE")
    else:
        G, result = gds.graph.project("my-graph", ["node"], {"EDGE":{"properties":["weight"]}})
    duration = end_time - start_time

    return G, duration, tot_vertex, tot_edges
    

def bfs(config, gds: GDS.GraphDataScience, G)->int:
    source_id = gds.find_node_id(["node"], {"nid":1})
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    gds.bfs.stream(G, sourceNode=source_id)        
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time
 
def pr(config, gds: GDS.GraphDataScience, G)->int:
    # figure out what max round is?
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    gds.pageRank.stream(G) 
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time 

# weakly connected components
def wcc(config, gds: GDS.GraphDataScience, G)->int:
    start_time = 0
    end_time = 0
    if not config["dataset"]["directed"]:
        start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
        gds.wcc.stream(G) 
        end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time

# community detection using label propagation
def cdlp(config,gds: GDS.GraphDataScience, G)->int:
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    gds.labelPropagation.stream(G) 
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time

# local cluster coefficient
def lcc(config, gds: GDS.GraphDataScience, G)->int:
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    gds.localClusteringCoefficient.stream(G) 
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time

# single source shortest paths
def sssp(config, gds: GDS.GraphDataScience, G)->int:
    source_id = gds.find_node_id(["node"], {"nid":1})
    start_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)    
    if config["dataset"]["weights"]:
       gds.allShortestPaths.dijkstra.stream(G, sourceNode=source_id, relationshipWeightProperty="weight") 
    else:
       gds.allShortestPaths.dijkstra.stream(G, sourceNode=source_id) 
    end_time = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
    return end_time - start_time

def main():
    # functional arguments position for the program
    # config_file_path id1 id2 algorithm dataset log_file
    
    config_yml = sys.argv[1]

    with open(config_yml, 'r') as yml_file:
        config = yaml.safe_load(yml_file)

    #sql params
    ids = [int(x.strip()) for x in config["config"]["ids"].split(",")]
    algos = [x.strip() for x in config["config"]["ids"].split(",")]
    id_algos = list(zip(ids, algos)) 
    nodes = config["confing"]["nodes"]

    log_file = config["config"]["log_file"]
    lf = open(log_file, "w+")

    neo_host = config["platform"]["host"]
    neo_port = config["platform"]["port"]
        
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
        gds = GDS.GraphDataScience(f"neo4j://{neo_host}:{neo_port}")
    except:
        lf.write("Error: could not connect to neo4j cluster\n")
        lf.close()
        conn.close()    
        quit(1)
    
    check_table(conn)
    
    [G, duration, vertex, edge] = load_data(gds, config, vertex_file, edge_file, 0)
    
    #vertex = graph_vertex_count(g)
    #edge = graph_edge_count(g)
    
    for entry in id_algos:
        log_metrics_sql(conn, entry[0], entry[1], dataset, "loading", duration, vertex, edge, nodes)
    
    func_d = {'bfs': bfs, 'pr':pr, 'wcc':wcc, 'cdlp':cdlp, 'lcc':lcc, 'sssp':sssp}
    
    for entry in id_algos:
        id_ = entry[0]
        algo = entry[1]
        
        requests.post('http://notifier:8080/starting')
        dur = func_d[algo](config, gds, G)
        requests.post('http://notifier:8080/stopping')
        
        if dur > 0:
            log_metrics_sql(conn, id_, algo, dataset, "runtime", dur, vertex, edge, nodes)

    lf.close()
    gds.close()
    conn.close()

main()
