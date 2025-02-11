use std::error::Error;
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct HostItem {
    pub url: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub interval: u64,
    pub hosts: Vec<HostItem>
}

impl Config {
    pub fn new() -> Self {
        Self { 
            interval: 1000, // Default 1 second interval
            hosts: vec![] 
        }
    }

    pub fn add_host(&mut self, host: HostItem) {
        self.hosts.push(host);
    }
    
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
