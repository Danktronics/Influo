# Influo
CI / CD with epic features

## Features

* Install with only **one binary**
* Deploy in environments like Docker with ease
* **Supports Linux and Windows.** Other platforms are untested but may work as long as they support Rust.
* Supports any language/framework that can be built and executed using the command line
* Pull from **any git repository** (some popular providers are github and gitlab)
* Build and deploy with logs all in one place
* Easy configuration using the ubiquitous JSON format
* **Very low footprint** and quick deployments thanks to Rust

## Wiki

Use the [Influo wiki](https://github.com/Danktronics/Influo/wiki) to get to deployment in minutes!

## Notes
Influo does **not** log with buffered stdout so if you use Python make sure to use the `-u` flag for unbuffered outputs.
