use std::time::Duration;
use tokio::time;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub last_check: DateTime<Utc>,
    pub is_healthy: bool,
    pub error_message: Option<String>,
}

pub struct Monitor {
    status: Arc<RwLock<HashMap<String, HealthStatus>>>,
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_monitoring(&self, host_name: String, url: String, interval_ms: u64) {
        let status = self.status.clone();
        
        tokio::spawn(async move {
            let client = reqwest::Client::new();
            let interval = Duration::from_millis(interval_ms);
            let mut interval_timer = time::interval(interval);

            loop {
                interval_timer.tick().await;
                
                let result = client.get(&url).send().await;
                let now = Utc::now();
                
                let health_status = match result {
                    Ok(response) => {
                        let is_healthy = response.status().is_success();
                        HealthStatus {
                            last_check: now,
                            is_healthy,
                            error_message: if is_healthy { 
                                None 
                            } else { 
                                Some(format!("HTTP {}", response.status()))
                            },
                        }
                    },
                    Err(e) => HealthStatus {
                        last_check: now,
                        is_healthy: false,
                        error_message: Some(e.to_string()),
                    },
                };

                status.write().insert(host_name.clone(), health_status);
            }
        });
    }

    pub fn get_status(&self) -> HashMap<String, HealthStatus> {
        self.status.read().clone()
    }
} 