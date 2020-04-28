use std::fs;
use std::io::ErrorKind;
use serde_json::{Result, Value};

fn main() -> Result<()> {
    println!("Influo is running!");

    let config: &str = &read_configuration()?;
    let v: Value = serde_json::from_str(config)?;
    println!("URL:\n{}", v["projects"]["url"]);
    Ok(())
}

fn read_configuration() -> Result<String> {
    let output = match fs::read_to_string("config.json") {
        Ok(file) => file,
        Err(error) => match error.kind() {
            ErrorKind::NotFound => match fs::read_to_string("./examples/example-config.json") {
                Ok(default) => default,
                Err(e) => panic!("Influo was unable to find both the config and the default config: {:?}", e),
            },
            other_error => panic!("Unable to open file: {:?}", other_error),
        },
    };
    Ok(output)
}
