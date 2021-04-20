use std::fs::File;
use std::io::BufReader;
use std::io::Write;

use anyhow::Error;
use serde_json::Value;

fn read_json_file(path: &str) -> Result<Value, Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    Ok(serde_json::from_reader(reader)?)
}

fn write_json_file(path: &str, json: &Value) -> Result<(), Error> {
    let mut file = File::create(&path)?;
    file.write_all(serde_json::to_string(json)?.as_bytes())?;
    Ok(())
}

pub fn read_configuration() -> Result<Value, Error> {
    read_json_file("config.json")
}

pub fn write_configuration(json: &Value) -> Result<(), Error> {
    write_json_file("config.json", json)
}