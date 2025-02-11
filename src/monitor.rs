use std::time::Duration;
use tokio::time;
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::{HashMap, VecDeque};
use chrono::{DateTime, Utc};
use std::io::{self, Write};

#[derive(Debug, Clone, PartialEq)]
pub enum StabilityState {
    Stable,
    Unstable,
    Unhealthy,
}

#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub last_check: DateTime<Utc>,
    pub is_healthy: bool,
    pub build_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub build_stability: BuildStability,
}

impl HealthStatus {
    pub fn get_state(&self) -> StabilityState {
        if !self.is_healthy {
            StabilityState::Unhealthy
        } else if self.build_stability.is_stable {
            StabilityState::Stable
        } else {
            StabilityState::Unstable
        }
    }
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
    previous_states: Arc<RwLock<HashMap<String, StabilityState>>>,
}

impl Monitor {
    pub fn new(stability_window: usize) -> Self {
        Self {
            status: Arc::new(RwLock::new(HashMap::new())),
            build_history: Arc::new(RwLock::new(HashMap::new())),
            stability_window,
            previous_states: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn notify_state_change(&self, host_name: &str, old_state: &StabilityState, new_state: &StabilityState) {
        if (old_state == &StabilityState::Stable || new_state == &StabilityState::Stable) 
            && old_state != new_state {
            print!("\x07"); // ASCII bell character
            io::stdout().flush().unwrap();
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
        let previous_states = self.previous_states.clone();
        
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

                        let health_status = HealthStatus {
                            last_check: now,
                            is_healthy,
                            build_at,
                            build_stability,
                            error_message: if is_healthy { 
                                None 
                            } else { 
                                Some(format!("HTTP {}", status_code))
                            },
                        };

                        let new_state = health_status.get_state();
                        let old_state = {
                            let mut states = previous_states.write();
                            let old = states.get(&host_name).cloned().unwrap_or(StabilityState::Unhealthy);
                            states.insert(host_name.clone(), new_state.clone());
                            old
                        };

                        if old_state != new_state {
                            if old_state == StabilityState::Stable || new_state == StabilityState::Stable {
                                print!("\x07"); // ASCII bell character
                                io::stdout().flush().unwrap();
                            }
                        }

                        health_status
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