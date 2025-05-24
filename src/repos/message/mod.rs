pub mod message;
pub mod neo4j_message;

pub use message::{AnyMessageRepository, MessageRepository};
pub use neo4j_message::Neo4jMessageRepository;
