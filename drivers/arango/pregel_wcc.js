
var pregel = require("@arangodb/pregel");
var params = {resultField: "component"};
var execution = pregel.start("wcc", "testGraph", params);
console.log(`\n!===${execution}===!\n`)
