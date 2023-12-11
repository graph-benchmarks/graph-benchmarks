
var pregel = require("@arangodb/pregel");
var params = {resultField: "rank"};
var execution = pregel.start("pagerank", "testGraph", params);
console.log(`\n!===${execution}===!\n`)
