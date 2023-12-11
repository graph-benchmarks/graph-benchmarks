schema.propertyKey("name").asText().ifNotExist().create();
schema.vertexLabel("src").properties("name").primaryKeys("name").ifNotExist().create();
schema.edgeLabel("rel").sourceLabel("src").targetLabel("src").ifNotExist().create();