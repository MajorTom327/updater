use std::error::Error;
use tokio;
use updater::config::Config;
use updater::monitor::Monitor;
use colored::*;
use std::io::{self, Write};
use chrono::{DateTime, Utc, Local};
use std::sync::Arc;

fn format_time_ago(time: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(time);
    
    if duration.num_seconds() < 60 {
        format!("{}s ago", duration.num_seconds())
    } else if duration.num_minutes() < 60 {
        format!("{}m ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h ago", duration.num_hours())
    } else {
        format!("{}d ago", duration.num_days())
    }
}

fn clear_console() {
    print!("\x1B[2J\x1B[1;1H");
    io::stdout().flush().unwrap();
}

fn print_status_dashboard(monitor: &Monitor) {
    clear_console();
    
    // Print header
    println!("{}", "Services Status Dashboard".bold());
    println!("{}", "=".repeat(110));
    
    let status = monitor.get_status();
    let mut hosts: Vec<_> = status.iter().collect();
    hosts.sort_by(|a, b| a.0.cmp(b.0)); // Sort by host name
    
    for (host_name, status) in hosts {
        let status_dot = if status.is_healthy {
            if status.build_stability.is_stable {
                "●".green()
            } else {
                "●".yellow()
            }
        } else {
            "●".red()
        };

        let build_info = status.build_at
            .map(|dt| dt.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "N/A".to_string());

        let last_check = format_time_ago(status.last_check);
        
        // Calculate the error message part
        let error_part = if let Some(error) = &status.error_message {
            format!(" - Error: {}...", error.chars().take(65).collect::<String>()).red()
        } else {
            "".clear()
        };

        // Calculate padding for right alignment of the timestamp
        let base_length = host_name.len() + build_info.len() + 5 + error_part.len(); // 5 for spaces and status dot
        let padding = if base_length < 100 {
            " ".repeat(100 - base_length)
        } else {
            " ".repeat(1)
        };

        println!("{} {} {} {}{}{} {}",
            status_dot,
            host_name.bold(),
            "at".dimmed(),
            build_info.blue(),
            error_part,
            padding,
            last_check.dimmed()
        );
    }
    
    println!("{}", "=".repeat(110));
    println!("{} Stable  {} Unstable  {} Unhealthy", 
        "●".green(),
        "●".yellow(),
        "●".red()
    );
}

async fn display_loop(monitor: Arc<Monitor>) {
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(1));
    loop {
        interval.tick().await;
        print_status_dashboard(&monitor);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::load_from_file("config.toml")?;
    let monitor = Arc::new(Monitor::new(config.stability_window));

    // Start monitoring threads for each host
    for host in &config.hosts {
        monitor.start_monitoring(
            host.name.clone(),
            host.url.clone(),
            config.interval
        ).await;
    }

    // Start the display loop in a separate task
    let display_monitor = monitor.clone();
    tokio::spawn(display_loop(display_monitor));

    // Keep the main task alive
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    }

    Ok(())
}
