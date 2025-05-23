# Development

Reservoir is currently intended for local development use. Below are the steps to set up and run the project.

## Prerequisites

- Rust (latest stable version)
- Docker (for Neo4j)

## Setup

### Step 1: Clone the Repository

```bash
git clone https://github.com/yourname/reservoir
cd reservoir
```

### Step 2: Start Neo4j with Docker Compose

```bash
docker-compose up -d
```

This starts Neo4j on the default `bolt://localhost:7687`.

### Step 3: Set Environment Variables

Create a `.env` file or export the following in your shell:

```env
RESERVOIR_PORT=3017
OPENAI_API_KEY=sk-...
NEO4J_URI=bolt://localhost:7687
NEO4J_USER=neo4j
NEO4J_PASSWORD=password
```

### Step 4: Run Reservoir

```bash
cargo run
```

Reservoir will now listen on `http://localhost:3017`.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.