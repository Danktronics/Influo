# Influo
CI / CD with epic features

## Features

* Install with one binary
* Deploy in many environments with ease (like docker)
* Supports Linux and Windows
* Supports any language/framework that can be built and executed using the command line
* Pull from any git repository (some popular providers are github and gitlab)
* Build and deploy with logs all in one place
* Easy configuration using the ubiquitous JSON format
* Quick deployments and low memory footprint thanks to Rust

## Wiki

Use the [Influo wiki](https://github.com/Danktronics/Influo/wiki) to get deploying in minutes!

## Notes
Influo does not log with **buffered** stdout so if you use Python make sure to put the `-u` flag for unbuffered outputs.
