#!/bin/bash
set -e

NEO4J_PASSWORD="${NEO4J_PASSWORD:-password}"

for file in migrations/*.cypher; do
  echo "Running $file"
  cypher-shell -u neo4j -p "$NEO4J_PASSWORD" --file "$file"
done
