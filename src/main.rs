use std::fs;

use serde_json::{Result, Value};

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
    let file_string = fs::read_to_string("./config.json")
        .expect("The config file was not found.");
    let output: &str = &file_string;    // I think this is type conversion from the original String
    return serde_json::from_str(output);
}
