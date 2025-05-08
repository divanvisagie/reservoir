use std::env;
use std::fs;
use std::path::Path;
use once_cell::sync::OnceCell;
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct ReservoirConfig {
    pub neo4j_uri: Option<String>,
    pub neo4j_user: Option<String>,
    pub neo4j_password: Option<String>,
}

static CONFIG: OnceCell<ReservoirConfig> = OnceCell::new();

fn load_config_file() -> ReservoirConfig {
    let path = Path::new("reservoir.toml");
    if path.exists() {
        let content = fs::read_to_string(path).unwrap_or_default();
        toml::from_str(&content).unwrap_or_default()
    } else {
        ReservoirConfig::default()
    }
}

fn get_config() -> &'static ReservoirConfig {
    CONFIG.get_or_init(|| load_config_file())
}

pub fn get_neo4j_uri() -> String {
    get_config().neo4j_uri.clone()
        .or_else(|| env::var("NEO4J_URI").ok())
        .unwrap_or_else(|| "bolt://localhost:7687".to_string())
}

pub fn get_neo4j_user() -> String {
    get_config().neo4j_user.clone()
        .or_else(|| env::var("NEO4J_USER").ok())
        .unwrap_or_else(|| "neo4j".to_string())
}

pub fn get_neo4j_password() -> String {
    get_config().neo4j_password.clone()
        .or_else(|| env::var("NEO4J_PASSWORD").ok())
        .unwrap_or_else(|| "password".to_string())
} 