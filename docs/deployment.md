# Deployment

Reservoir is currently intended for local development use. Below are the steps to deploy it locally.

## Prerequisites

- Rust (latest stable version)
- Docker (for Neo4j)

## Steps

1. **Clone the Repository**:
2. **Start Neo4j**

   You have two options for running Neo4j locally:

   **a) With Docker Compose (recommended for most users):**

   ```bash
   docker-compose up -d
   ```

   This starts Neo4j on the default `bolt://localhost:7687`.

   **b) With Homebrew (runs Neo4j as a macOS service):**

   If you use Homebrew, you can install and run Neo4j as a background service that starts automatically when your computer boots:

   ```bash
   brew install neo4j
   brew services start neo4j
   ```

   This will also start Neo4j on `bolt://localhost:7687` and ensure it is always running in the background.

3. **Set Environment Variables**:

   Create a `.env` file or export the following in your shell:

   ```env
   RESERVOIR_PORT=3017
   OPENAI_API_KEY=sk-...
   NEO4J_URI=bolt://localhost:7687
   NEO4J_USER=neo4j
   NEO4J_PASSWORD=password
   RSV_OPENAI_BASE_URL=https://api.openai.com/v1/chat/completions
   RSV_OLLAMA_BASE_URL=http://localhost:11434/v1/chat/completions
   ```

   > Note: All environment variables except `OPENAI_API_KEY` have sensible defaults if not set. `OPENAI_API_KEY` is required.

4. **Run Reservoir (manually)**:

   ```bash
   cargo run
   ```

   Reservoir will now listen on `http://localhost:3017`.

5. **(Optional) Install Reservoir as a macOS Service**

   You can run Reservoir automatically as a background service using `launchctl` and the provided LaunchAgent plist.

   **To install and start the service:**

   ```bash
   make install-service
   ```

   This will:
   - Copy `scripts/com.sectorflabs.reservoir.plist` to `~/Library/LaunchAgents/`
   - Load the service using `launchctl`
   - Start Reservoir in the background (logs will be in `/tmp/reservoir.log` and `/tmp/reservoir.err`)

   **To stop and remove the service:**

   ```bash
   make uninstall-service
   ```

   **To manually control the service with `launchctl`:**

   - Start the service:
     ```bash
     launchctl start com.sectorflabs.reservoir
     ```
   - Stop the service:
     ```bash
     launchctl stop com.sectorflabs.reservoir
     ```
   - View service logs:
     ```bash
     tail -f /tmp/reservoir.log
     tail -f /tmp/reservoir.err
     ```

   > **Note:** Ensure your binary path in the plist (`/Users/divan/.cargo/bin/rsrvr`) matches where your built binary is located. Adjust the plist if needed.
