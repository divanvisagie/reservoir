DROP INDEX embeddingEmbeddings;
CREATE VECTOR INDEX embeddingEmbeddings
FOR (n:Embedding)
ON (n.embedding)
OPTIONS {
  indexConfig: {
    `vector.dimensions`: 1536,
    `vector.similarity_function`: 'cosine'
  }
};
