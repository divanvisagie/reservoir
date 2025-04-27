# Deployment

Reservoir is currently intended for local development use. Below are the steps to deploy it locally.

## Prerequisites

- Rust (latest stable version)
- Docker (for Neo4j)

## Steps

1. **Clone the Repository**:

   ```bash
   git clone https://github.com/yourname/reservoir
   cd reservoir
   ```

2. **Start Neo4j with Docker Compose**:

   ```bash
   docker-compose up -d
   ```

   This starts Neo4j on the default `bolt://localhost:7687`.

3. **Set Environment Variables**:

   Create a `.env` file or export the following in your shell:

   ```env
   RESERVOIR_PORT=3017
   OPENAI_API_KEY=sk-...
   NEO4J_URI=bolt://localhost:7687
   NEO4J_USER=neo4j
   NEO4J_PASSWORD=password
   ```

4. **Run Reservoir**:

   ```bash
   cargo run
   ```

   Reservoir will now listen on `http://localhost:3017`.