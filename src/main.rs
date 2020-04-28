use std::fs;
use failure::Error;
use serde_json::Value;

fn main() -> Result<(), Error> {
    println!("Influo is running!");

    // Load Configuration
    let config: Value = read_configuration()?;
    println!("URL:\n{}", config["projects"]["url"]);

    let update_interval: Value = config["update_interval"];
    if update_interval.is_null() || !update_interval.is_number() {
        setup_updater_thread(30);
    } else {
        let interval: Option<u64> = update_interval.as_u64();
        if interval.is_none() || interval > u32::MAX {
            throw!("The integer provided exceeded the u32 max");
        }
        setup_updater_thread(update_interval.unwrap());   
    }

    Ok(())
}

/// Interval is in milliseconds
fn setup_updater_thread(interval: u32) {
    thread::spawn(|| {
        thread::sleep(Duration::from_millis(interval));
        println!("Run here")
    });
}

fn read_configuration() -> Result<Value, Error> {
    let raw_data: String = fs::read_to_string("config.json")?;
    Ok(serde_json::from_str(&raw_data)?)
}
