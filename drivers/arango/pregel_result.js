
var pregel = require("@arangodb/pregel")
const execution = "!!!----!!!"  // replace this with the execution time
console.log(`\n!===${pregel.status(execution)["status"]}===!\n`)
console.log(`\n!===#${pregel.status(execution)["running_time"]}===#!\n`)
