arangoimport --file "/attached/ar_vertex.csv" --type csv --collection "vertex" --translate "vertex=_key" --create-collection true --server.password ""
arangoimport --file "/attached/ar_edges.csv" --type csv --collection "edges" --create-collection true --create-collection-type edge --translate "src=_from" --translate "dst=_to" --server.password ""
