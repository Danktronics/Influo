use std::fs;
use failure::Error;
use serde_json::Value;

fn main() -> Result<(), Error> {
    println!("Influo is running!");

    let config: Value = read_configuration()?;
    println!("URL:\n{}", config["projects"]["url"]);
    Ok(())
}

fn read_configuration() -> Result<Value, Error> {
    let raw_data: String = fs::read_to_string("config.json")?;
    Ok(serde_json::from_str(&raw_data)?)
}
