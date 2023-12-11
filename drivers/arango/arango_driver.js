const yaml = require("yaml");
const fs = require("fs");
const pg = require("pg");
const child_process = require("child_process");

async function check_table(client) {
  const check_table_query =
    "SELECT EXISTS (SELECT 1 FROM information_schema.tables WHERE table_name = 'gn_test') AS table_existence";

  const res = await client.query(check_table_query);
  const tableExists = res.rows[0].table_existence;

  // create a table
  if (!tableExists) {
    const create_table_query =
      "CREATE TABLE gn_test(id INTEGER, algo VARCHAR(256), dataset VARCHAR(256), type VARCHAR(256), time INTEGER, vertex INTEGER, edge INTEGER, nodes INTEGER)";
    await client.query(create_table_query);
  }
}

async function log_metrics_sql(
  client,
  log_id,
  algo,
  dataset,
  type_,
  time,
  vertex,
  edge,
  nodes,
) {
  const log_query =
    "INSERT INTO gn_test(id, algo, dataset, type, time, vertex, edge, nodes) VALUES($1, $2, $3, $4, $5, $6, $7, $8)";
  await client.query(log_query, [
    log_id,
    algo,
    dataset,
    type_,
    time,
    vertex,
    edge,
    nodes,
  ]);
}

async function get_vertex_edge_count() {}

// can only be run after data has been loaded
function run_arangosh(arango_host, arango_port, arango_user, command) {
  // creating graph
  const endpoint = `--server.endpoint tcp://${arango_host}:${arango_port}`;
  const user = `--server.username ${arango_user}`;
  const auth = `--server.authentication false`;
  const password = `--server.password ""`
  const com = `arangosh ${endpoint} ${user} ${auth} ${password} ${command}`
  //console.log(com)
  child_process.execSync(com);
}

function get_execution_result(output_file) {
  const resf = fs.readFileSync(output_file, "utf8");
  // regular expression match
  const regexPattern = /!===[\s\S]*?===!/;
  const match = resf.match(regexPattern);
  const match_str = match[0];

  const regexPattern2 = /!===#(.*?)#===!/;
  const match2 = resf.match(regexPattern2);
  let ret2 = ""

  if (match2 !== null){
        const match_str2 = match2[0];
        ret2 = match_str2.replace("!===#", "").replace("#===!", "");
  } else {
        ret2 =  ""
  }
  const ret1 = match_str.replace("!===", "").replace("===!", ""); // solution
  return [ret1, ret2];
}

function remove_execution_code(result_code_file, code) {
  const rf = fs.readFileSync(result_code_file, "utf8");
  const new_rf = rf.replace(code, "!!!----!!!");
  fs.writeFileSync(result_code_file, new_rf);
}

function add_execution_code(result_code_file, code) {
  const rf = fs.readFileSync(result_code_file, "utf8");
  const new_rf = rf.replace("!!!----!!!", code);
  fs.writeFileSync(result_code_file, new_rf);
}

async function wait_until_exec_result() {
  while (True) {
    await new Promise((resolve) => setTimeout(resolve, 1000));
    exec_result = get_execution_result();
    if (exec_result[0] === "done") {
      return exec_result[1];
    }
  }
}

async function bfs() {}

async function pr(
  arango_host,
  arango_port,
  arango_user,
  output_file,
  res_code_file,
) {
  const command = `< pregel_pr.js > ${output_file}`;
  run_arangosh(arango_host, arango_port, arango_user, command);
  const code = get_execution_result(output_file)[0];
  add_execution_code(res_code_file, code);
  const dur = wait_until_exec_result();
  remove_execution_code(res_code_file, code);
  return dur;
}

async function wcc(
  arango_host,
  arango_port,
  arango_user,
  output_file,
  res_code_file,
) {
  const command = `< pregel_wcc.js > ${output_file}`;
  run_arangosh(arango_host, arango_port, arango_user, command);
  const code = get_execution_result(output_file)[0];
  add_execution_code(res_code_file, code);
  const dur = wait_until_exec_result();
  remove_execution_code(res_code_file, code);
  return dur;
}

async function cdlp(
  arango_host,
  arango_port,
  arango_user,
  output_file,
  res_code_file,
) {
  const command = `< pregel_cdlp.js > ${output_file}`;
  run_arangosh(arango_host, arango_port, arango_user, command);
  const code = get_execution_result(output_file)[0];
  add_execution_code(res_code_file, code);
  const dur = wait_until_exec_result();
  remove_execution_code(res_code_file, code);
  return dur;
}

async function lcc() {}

async function sssp(
  arango_host,
  arango_port,
  arango_user,
  output_file,
  res_code_file,
) {
  const command = `< pregel_sssp.js > ${output_file}`;
  run_arangosh(arango_host, arango_port, arango_user, command);
  const code = get_execution_result(output_file)[0];
  add_execution_code(res_code_file, code);
  const dur = wait_until_exec_result();
  remove_execution_code(res_code_file, code);
  return dur;
}

async function main() {
  config_file = process.argv[2];

  const cf = fs.openSync(config_file);
  const fconfig = fs.readFileSync(cf, "utf8");
  const config = yaml.parse(fconfig);
  fs.closeSync(cf);

  //sql params
  var ids = config["config"]["ids"].toString().split(",");
  ids = ids.map((id) => parseInt(id));
  algos = config["config"]["algos"].split(",");

  var id_algos = ids.map((id, ind) => [id, algos[ind]]);

  nodes = config["config"]["nodes"];
  log_file = config["config"]["log_file"];
  lf = fs.openSync(log_file, "w+");

  arango_host = config["platform"]["host"];
  arango_port = config["platform"]["port"];
  arango_user = config["platform"]["user"];

  pg_host = config["postgres"]["host"];
  pg_db = config["postgres"]["db"];
  pg_port = config["postgres"]["port"];
  user_ps = config["postgres"]["ps"];
  pg_user = config["postgres"]["user"];

  dataset = config["dataset"]["name"];
  vertex_file = config["dataset"]["vertex"];
  edge_file = config["dataset"]["edges"];

  // connect to postgres
  const pg_URI = `postgresql://${pg_user}:${user_ps}@${pg_host}:${pg_port}/${pg_db}`;
  const client = new pg.Client(pg_URI);

  try {
    await client.connect();
  } catch {
    fs.writeFileSync(lf, "Error: could not connect to postgres");
    process.exit();
  }
  // connect to arango

  await check_table(client);

  /*
for (entry of id_algos) {
    const tid = entry[0];
    const talgo = entry[1];
    await log_metrics_sql(
      client,
      tid,
      talgo,
      dataset,
      "loading",
      dur,
      vertex,
      edge,
      nodes,
    );
  }
*/
  func_d = { bfs, pr, wcc, cdlp, lcc, sssp };
  const output_file = "arangosh_output.txt"
  const result_file = "pregel_result.js"

  // create a graph
  create_graph_command = "< pregel_create_graph.js"
  run_arangosh(arango_host, arango_port, arango_user, create_graph_command);

  for (entry of id_algos) {
    const tid = entry[0];
    const algo = entry[1];
    await fetch("http://notifier:8080/starting")
    dur = func_d[algo](arango_host, arango_port, arango_user, result_file, output_file);
    dur = parseInt(parseFloat(dur) * 1000)
    await fetch("http://notifier:8080/stopping")
    log_metrics_sql(client, tid, algo, dataset, "runtime", dur, 0, 0, nodes)
  }

  fs.closeSync(lf);
  await client.end();
}

main();
