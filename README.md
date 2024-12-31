# Raytx

Raytx is a powerful tool for performing token swap operations on Raydium and Pump.fun, providing both CLI and API interfaces.

## Features

- Command-line interface for quick swaps
- RESTful API service for programmatic access
- Support for buy/sell operations
- Integration with Jito for faster transactions
- Percentage-based selling options

## Project Dependencies

Before getting started, ensure that the following software is installed on your system:

- [Rust](https://www.rust-lang.org/) version 1.8 or higher.


## Build
```
cargo build -r
```
This will generate an executable file raytx, located in the `target/release/raytx`.

## Using the Command-Line Tool
### Buy
```
raytx swap <mint> buy --amount-in=<amount-in>
```
### Sell
```
# sell 50%
raytx swap <mint> sell --amount-in-pct=0.5

# sell all, close wallet ata when sell all
raytx swap <mint> sell --amount-in-pct=1

# Sell 1000
raytx swap <mint> sell --amount-in=1000
```
Replace <mint> with the address of the token you want to swap, and <amount-in> with the quantity|<amount-in-pct> with the percentage you want to swap.

### Jito
Use `--jito` to speed up swap.
[Read more](./docs/jito.md)

## Using swap api

To start the daemon, use the following command:
```bash
raytx daemon
```
[More information in the documentation](./docs/api.md)


# Contributing
Contributions to this project are welcome. If you have any questions or suggestions, feel free to raise an issue.

# License
This project is licensed under the MIT License.
