use std::error::Error;
use tokio;
use updater::config::Config;
use updater::monitor::Monitor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::load_from_file("config.toml")?;
    let monitor = Monitor::new();

    // Start monitoring threads for each host
    for host in &config.hosts {
        monitor.start_monitoring(
            host.name.clone(),
            host.url.clone(),
            config.interval
        ).await;
    }

    // Example: Periodically print status
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        let status = monitor.get_status();
        println!("Current status: {:#?}", status);
    }

    Ok(())
}
