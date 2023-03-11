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

## Understanding Rocust
Rocust is a load testing tool that allows you to define your user behaviour with Rust code. It is inspired by [Locust](https://locust.io/), a Python load testing tool. Rocust is built on top of [Tokio](https://tokio.rs/), An asynchronous Rust runtime. Rocust is designed to be used as a library, and can be integrated into your Rust project. Rocust is also designed to be used as a standalone tool, and can be used to swarm your system with millions of simultaneous users.

Rocust produces metrics similar to Locust. It also produces prometheus metrics, which can be used to monitor your system. Set up your prometheus job to scrape Rocust metrics on your defined port. Ideally set your scrape interval to 1 second to get the most accurate metrics.

## Achieving the same Results with Rocust built-in result-system and PromQL
```sh	
# total requests sent
sum (rocust_requests_total)

# total failures
sum (rocust_failures_total)

# total errors
sum (rocust_errors_total)

# total requests per user type
sum by (user_name) (rocust_requests_total)

# total requests per user type and endpoint name
sum by (user_name, endpoint_name) (rocust_requests_total)

# total requests per endpoint name and type
sum by (endpoint_name, endpoint_type) (rocust_requests_total)

# maximum response time for the last 60 minutes
max by (max_over_time(rocust_response_time[60m]))

# maximum response time for a certian user type for the last 60 minutes
max by (user_name) (max_over_time(rocust_response_time[60m]))

# maximum response time for a certian endpoint name for the last 60 minutes
max by (endpoint_name) (max_over_time(rocust_response_time[60m]))

# minimum is analogical to maximum

# average response time
avg(rocust_response_time)
```
Obviously, you can use PromQL to get much more comlex results. For more information, check out the [Prometheus documentation](https://prometheus.io/docs/prometheus/latest/querying/basics/).


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
- [X] Prometheus metrics (prio.)

## Rust version 
1.67.1

## Contributors
* Jad K. Haddad <jadkhaddad@gmail.com>

## License & copyright
© 2023 Jad K. Haddad
Licensed under the [MIT License](LICENSE)
