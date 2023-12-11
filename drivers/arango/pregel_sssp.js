
var pregel = require("@arangodb/pregel");
var params = {source: "vertex/1"};
var execution = pregel.start("sssp", "testGraph", params);
console.log(`\n!===${execution}===!\n`)
