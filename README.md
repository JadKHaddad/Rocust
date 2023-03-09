# Rocust: An open source load testing tool.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Define user behaviour with Rust code, and swarm your system with millions of simultaneous users.

![alt text](https://github.com/JadKHaddad/rocust/blob/main/assets/logo_long.png?raw=true)

## Status
In development

## Run the example
Set up your ```test_config``` in ```dev/src/main.rs``` and run the following command:
```sh
cargo run -p dev
```

## TODO
- [X] TestConfig from CLI
- [X] TestConfig from file: YAML, JSON
- [X] User summary
- [ ] Stop condition parser to create a stop condition from a string
- [ ] Documentation
- [ ] Examples
- [ ] Tests
- [ ] Web interface
- [ ] Master/workers architecture (prio.)
- [ ] Other Features
- [ ] Prometheus metrics (prio.)

## Rust version 
1.67.1

## Contributors
* Jad K. Haddad <jadkhaddad@gmail.com>

## License & copyright
© 2023 Jad K. Haddad
Licensed under the [MIT License](LICENSE)
