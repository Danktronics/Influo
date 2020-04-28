use std::fs;
use std::io::ErrorKind;
use serde_json::{Result, Value};

fn main() -> Result<()> {
    println!("Influo is running!");

    let config: Value = read_configuration()?;
    println!("URL:\n{}", config["projects"]["url"]);
    Ok(())
}

fn read_configuration() -> Result<Value> {
    let raw_data: str = fs::read_to_string("config.json")?;
    serde_json::from_str(raw_data)?
}
