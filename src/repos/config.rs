use std::env;
use std::fs;
use std::path::PathBuf;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use dirs_next::config_dir;

#[derive(Debug, Deserialize, Serialize)]
pub struct ReservoirConfig {
    #[serde(default = "default_neo4j_uri")]
    pub neo4j_uri: Option<String>,
    #[serde(default = "default_neo4j_user")]
    pub neo4j_user: Option<String>,
    #[serde(default = "default_neo4j_password")]
    pub neo4j_password: Option<String>,
    #[serde(default = "default_reservoir_port")]
    pub reservoir_port: Option<u16>,
}

fn default_neo4j_uri() -> Option<String> {
    Some("bolt://localhost:7687".to_string())
}
fn default_neo4j_user() -> Option<String> {
    Some("neo4j".to_string())
}
fn default_neo4j_password() -> Option<String> {
    Some("password".to_string())
}
fn default_reservoir_port() -> Option<u16> {
    Some(3017)
}

impl Default for ReservoirConfig {
    fn default() -> Self {
        ReservoirConfig {
            neo4j_uri: default_neo4j_uri(),
            neo4j_user: default_neo4j_user(),
            neo4j_password: default_neo4j_password(),
            reservoir_port: default_reservoir_port(),
        }
    }
}

static CONFIG: OnceCell<ReservoirConfig> = OnceCell::new();

fn get_reservoir_config_path() -> PathBuf {
    let mut path = config_dir().unwrap_or_else(|| env::current_dir().unwrap());
    path.push("reservoir");
    path.push("reservoir.toml");
    path
}

fn load_config_file() -> ReservoirConfig {
    let path = get_reservoir_config_path();
    println!("Loading config from {}", path.display());
    if path.exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        toml::from_str(&content).unwrap_or_default()
    } else {
        // Create the directory and file, and write defaults
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let default = ReservoirConfig::default();
        let toml_str = toml::to_string_pretty(&default).unwrap_or_default();
        let _ = fs::write(&path, toml_str);
        default
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

pub fn get_reservoir_port() -> u16 {
    get_config().reservoir_port
        .or_else(|| env::var("RESERVOIR_PORT").ok().and_then(|v| v.parse().ok()))
        .unwrap_or(3017)
} 