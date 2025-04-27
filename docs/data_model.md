# Data Model

Reservoir uses Neo4j to store conversations as a graph. Below is an overview of the data model.

## Nodes
### MessageNode:
Represents a single message (user or assistant).

| Property     | Description                                                                 |
|--------------|-----------------------------------------------------------------------------|
| `trace_id`   | Unique per request/response pair.                                           |
| `partition`  | Logical namespace from the request URL, typically set to the system username (`$USER`). |
| `instance`   | Specific context within a partition from the URL, typically set to the application name. |
| `role`       | Role of the message (`user` or `assistant`).                                |
| `content`    | The text content of the message.                                            |
| `timestamp`  | When the message was created.                                               |
| `embedding`  | Vector representation of the message.                              |
| `url`        | Optional URL associated with the message.                                   |

## Relationships

### RESPONDED_WITH
Links a user message to the corresponding assistant response. This relationship is permanent and ensures data integrity by preserving the original conversation structure.

### SYNAPSE
Links semantically similar messages based on vector similarity. Synapses are dynamic and flexible relationships between messages. The system can create, update, or remove synapses at any time based on the current state of the graph or new data. This ensures that the relationships between messages remain relevant and up-to-date.

- Synapses are initially created sequentially, documenting the continuous flow of conversation over time.
- If the similarity between consecutive messages drops below the threshold (0.85), the synapse is removed, indicating a topic change in the conversation.
- Synapses are created between messages with high semantic similarity, using vector similarity scores.
- The system can dynamically adjust synapses as new messages are added or as the context evolves.

## Example Graph

```plaintext
(User Message)-[:RESPONDED_WITH]->(Assistant Message)
(User Message)-[:SYNAPSE {score: 0.9}]->(Similar User Message)
```

## Vector Index

Reservoir uses a vector index (`messageEmbeddings`) in Neo4j to enable efficient similarity searches. This index is based on cosine similarity and supports operations like finding semantically similar messages.

## Why Neo4j?

Neo4j's graph capabilities allow for:
- Modeling relationships between conversations and messages.
- Advanced querying for context enrichment.
- Leveraging built-in vector search features for semantic similarity.
- Dynamically connecting semantically similar messages through synapses.

## Partition and Instance

- **Partition**: Typically set to the system username (`$USER`). This allows grouping messages by the user running the system.
- **Instance**: Typically set to the application name. This allows grouping messages by the specific application or context within the user's environment.

This convention ensures that messages are logically organized and scoped to the user and application, enabling efficient querying and context enrichment.

## Fixed and Dynamic Relationships

One of the core concepts of the data model is the distinction between fixed and dynamic relationships:

- **Fixed Relationships**:
  - **MessageNode**: Represents a single message and its properties, which remain immutable once created.
  - **RESPONDED_WITH**: Links a user message to the corresponding assistant response. This relationship is permanent and ensures data integrity by preserving the original conversation structure.

- **Dynamic Relationships**:
  - **SYNAPSE**: Represents semantically similar messages. These relationships are dynamic and flexible, allowing the system to create, update, or remove them as needed. This flexibility supports learning systems and ensures that the graph remains relevant and up-to-date as new data is added.

This distinction is key to the system architecture, as it ensures data integrity for core elements while allowing adaptability and learning where required.