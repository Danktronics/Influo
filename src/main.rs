use std::fs;

use serde_json::{Result, Value, Error};

fn main() -> Result<()> {
    println!("Influo is running!");

    let v: Value = match read_configuration() {
        Ok(v) => v,
        Err(e) => return Err(e),
    };
    println!("URL:\n{}", v["projects"]["url"]);
    Ok(())
}

fn read_configuration() -> Result<Value> {
    let output: &str = &fs::read_to_string("./config.json")
        .expect("The config file was not found.");
    return serde_json::from_str(output);
}
