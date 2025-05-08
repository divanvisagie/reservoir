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
    pub subcmd: Option<SubCommands>
}

#[derive(Parser, Debug)]
pub enum SubCommands {
    /// Set or get default configuration values with your config.toml.
    Config(ConfigSubCommand),
    Start(StartSubCommand),
    /// Export all message nodes as JSON
    Export,
}

#[derive(Parser, Debug)]
#[command(author, version, about = "Start the Reservoir proxy", long_about = None)]
pub struct StartSubCommand {
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
