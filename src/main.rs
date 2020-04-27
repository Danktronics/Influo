use std::fs;
use std::io;

use serde_json::Value;

fn main() {
    println!("Influo is running!");

    println!(read_configuration())
}

fn read_configuration() -> Result<Value, io:Error> {
    let opened_file = fs::read_to_string("./config.json");
    if opened_file.is_err() {
        return Err(opened_file);
    }

    return serde_json::from_str(opened_file);
}
