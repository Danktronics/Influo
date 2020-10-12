# Influo
CI / CD with epic features

## Features

* Install with only **one binary**
* Deploy in many environments with ease
* **Supports Linux and Windows.** Other platforms are untested but may work.
* Supports any language/framework that can be built and executed using the command line
* Pull from **any git repository** as long as Git is installed and setup
* Build and deploy with logs all in one place
* Easy configuration using the ubiquitous JSON format
* **Very low footprint** and quick deployments thanks to Rust

## Wiki

Use the [Influo wiki](https://github.com/Danktronics/Influo/wiki) to get to deployment in minutes!

## Notes
Influo does **not** log with **buffered** stdout so if you use Python make sure to use the `-u` flag for unbuffered outputs.
