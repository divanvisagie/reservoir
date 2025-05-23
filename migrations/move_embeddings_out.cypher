MATCH (m:MessageNode)
WHERE NOT (m)-[:HAS_EMBEDDING]->(:Embedding)
CREATE (e:Embedding {
    embedding: m.embedding, 
    model: 'text-embedding-ada-002',
    partition: m.partition,
    instance: m.instance
})
CREATE (m)-[:HAS_EMBEDDING]->(e)
RETURN id(m) AS nodeId, id(e) AS embeddingId
