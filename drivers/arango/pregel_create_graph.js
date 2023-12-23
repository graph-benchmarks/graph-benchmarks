// has to be run from arangosh

var graph_module = require("@arangodb/general-graph");
var graph = graph_module._create("testGraph");

// adding vertex collections
graph._addVertexCollection("vertex");
graph = graph_module._graph("testGraph");

// edge definition
var rel = graph_module._relation("edges", ["vertex"], ["vertex"]);
graph._extendEdgeDefinitions(rel);
graph = graph_module._graph("testGraph");
