
var pregel = require("@arangodb/pregel");
var params = {resultField: "community"};
var execution = pregel.start("labelpropagation", "testGraph", params);
console.log(`\n!===${execution}===!\n`)
