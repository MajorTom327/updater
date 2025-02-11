use std::error::Error;


use updater::config::Config;

fn main() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");

    let config = Config::load_from_file("config.toml")?;

    println!("Config: {:?}", config);

    Ok(())
}
