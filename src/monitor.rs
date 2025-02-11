use std::time::Duration;
use tokio::time;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub last_check: DateTime<Utc>,
    pub is_healthy: bool,
    pub build_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub build_stability: BuildStability,
}

#[derive(Debug, Clone)]
pub struct BuildStability {
    pub is_stable: bool,
    pub recent_builds: Vec<DateTime<Utc>>,
}

pub struct Monitor {
    status: Arc<RwLock<HashMap<String, HealthStatus>>>,
    build_history: Arc<RwLock<HashMap<String, VecDeque<DateTime<Utc>>>>>,
    stability_window: usize,
}

impl Monitor {
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(HashMap::new())),
            build_history: Arc::new(RwLock::new(HashMap::new())),
            stability_window: 5, // Consider last 5 builds for stability
        }
    }

    fn check_build_stability(&self, host_name: &str, current_build: DateTime<Utc>) -> BuildStability {
        let mut history = self.build_history.write();
        let build_queue = history.entry(host_name.to_string())
            .or_insert_with(|| VecDeque::with_capacity(self.stability_window));

        // Add new build timestamp to history
        build_queue.push_back(current_build);
        
        // Keep only the last N builds
        while build_queue.len() > self.stability_window {
            build_queue.pop_front();
        }

        // Check if all recent builds are the same
        let is_stable = build_queue.len() == self.stability_window && 
            build_queue.iter().all(|&build| build == current_build);

        BuildStability {
            is_stable,
            recent_builds: build_queue.iter().cloned().collect(),
        }
    }

    pub async fn start_monitoring(&self, host_name: String, url: String, interval_ms: u64) {
        let status = self.status.clone();
        let build_history = self.build_history.clone();
        let stability_window = self.stability_window;
        
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
                        let status_code = response.status();
                        let is_healthy = status_code.is_success();
                        let (build_at, build_stability) = if is_healthy {
                            match response.json::<serde_json::Value>().await {
                                Ok(json) => {
                                    let build_at = json.get("buildAt")
                                        .and_then(|v| v.as_str())
                                        .and_then(|date_str| DateTime::parse_from_rfc3339(date_str).ok())
                                        .map(|dt| dt.with_timezone(&Utc));
                                    
                                    let stability = if let Some(ba) = build_at {
                                        let mut history = build_history.write();
                                        let build_queue = history.entry(host_name.clone())
                                            .or_insert_with(|| VecDeque::with_capacity(stability_window));

                                        build_queue.push_back(ba);
                                        while build_queue.len() > stability_window {
                                            build_queue.pop_front();
                                        }

                                        let is_stable = build_queue.len() == stability_window && 
                                            build_queue.iter().all(|&build| build == ba);

                                        BuildStability {
                                            is_stable,
                                            recent_builds: build_queue.iter().cloned().collect(),
                                        }
                                    } else {
                                        BuildStability {
                                            is_stable: false,
                                            recent_builds: Vec::new(),
                                        }
                                    };
                                    
                                    (build_at, stability)
                                },
                                Err(e) => {
                                    println!("Failed to parse JSON: {}", e);
                                    (None, BuildStability {
                                        is_stable: false,
                                        recent_builds: Vec::new(),
                                    })
                                }
                            }
                        } else {
                            (None, BuildStability {
                                is_stable: false,
                                recent_builds: Vec::new(),
                            })
                        };

                        HealthStatus {
                            last_check: now,
                            is_healthy,
                            build_at,
                            build_stability,
                            error_message: if is_healthy { 
                                None 
                            } else { 
                                Some(format!("HTTP {}", status_code))
                            },
                        }
                    },
                    Err(e) => HealthStatus {
                        last_check: now,
                        is_healthy: false,
                        build_at: None,
                        build_stability: BuildStability {
                            is_stable: false,
                            recent_builds: Vec::new(),
                        },
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