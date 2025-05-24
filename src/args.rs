use clap::{command, Parser};

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about = r###"
Reservoir is a transparent proxy for any OpenAI-compatible API. It captures all your AI conversations and stores them in a Neo4j graph, turning every interaction into a searchable, self-growing knowledge base.

Think of it as a personal neural lake that evolves into an intelligent assistant with memory:
- Capture: Every prompt and response is logged, building a rich history.
- Dynamic Context Enrichment: Automatically adds relevant past conversations to new prompts, giving the AI a memory-like experience via graph relationships and vector search.
- Self-building: Your interactions continuously enrich the knowledge base.
- Plug-and-Play: Drop it in front of your OpenAI-compatible app — no client code changes needed.
"###
)]
pub struct Args {
    #[command(subcommand)]
    pub subcmd: Option<SubCommands>,
}

#[derive(Parser, Debug)]
pub enum SubCommands {
    /// Set or get default configuration values with your config.toml.
    Config(ConfigSubCommand),
    Start(StartSubCommand),
    /// Export all message nodes as JSON
    Export,
    /// Import message nodes from a JSON file
    Import(ImportSubCommand),
    /// View last x messages in the default partition/instance
    View(ViewSubCommand),
    /// Search messages by keyword or semantic similarity
    Search(crate::commands::search::SearchSubCommand),
    /// Ingest a message from stdin as a user MessageNode
    Ingest(IngestSubCommand),
    /// Replay embeddings process
    Replay(ReplaySubCommand),
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Start the Reservoir proxy", long_about = None)]
pub struct StartSubCommand {
    /// Ollama mode which sets up on same default port as ollama
    /// useful for using as a proxy for clients that don't support
    /// setting a url
    #[arg(short, long)]
    pub ollama: bool,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Set or get configuration values", long_about = None)]
pub struct ConfigSubCommand {
    /// Set a configuration value. Use the format key=value.
    /// `cgip config --set model=gpt-4-turbo`
    #[arg(short, long)]
    pub set: Option<String>,

    /// Get your current configuration value.
    /// `cgip config --get model`
    #[arg(short, long)]
    pub get: Option<String>,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Import message nodes from a JSON file", long_about = None)]
pub struct ImportSubCommand {
    /// Path to the JSON file to import
    pub file: String,
}
// ----------------------------------------
#[derive(Parser, Debug)]
#[command(author, version, about = "View last x messages", long_about = None)]
pub struct ViewSubCommand {
    /// Number of messages to display
    pub count: usize,
    /// Partition to view (defaults to "default")
    #[arg(short, long)]
    pub partition: Option<String>,
    /// Instance to view (defaults to partition)
    #[arg(short, long)]
    pub instance: Option<String>,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Ingest a message from stdin as a user MessageNode", long_about = None)]
pub struct IngestSubCommand {
    /// Partition to save the message in (defaults to "default")
    #[arg(short, long)]
    pub partition: Option<String>,
    /// Instance to save the message in (defaults to partition)
    #[arg(short, long)]
    pub instance: Option<String>,
    /// Role to assign to the message (defaults to "user")
    #[arg(long)]
    pub role: Option<String>,
}

//replay subcommand
#[derive(Parser, Debug)]
#[command(author, version, about = "Replay embeddings process", long_about = None)]
pub struct ReplaySubCommand {
    /// Partition to replay (defaults to "default")
    pub model: Option<String>,
}
