
var pregel = require("@arangodb/pregel")
const execution = "!!!----!!!"  // replace this with the execution time
console.log(`\n!===${pregel.status(execution)["state"]}===!\n`)
console.log(`\n!===#${pregel.status(execution)["totalRuntime"]}===#!\n`)
