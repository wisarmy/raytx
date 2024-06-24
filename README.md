# Raytx

Raytx is a command-line tool written in Rust for performing token swap operations on Raydium.

## Project Dependencies

Before getting started, ensure that the following software is installed on your system:

- [Node.js](https://nodejs.org/)
- [Rust](https://www.rust-lang.org/)


## Build
```
cargo build -r
```
This will generate an executable file raytx, located in the target/release/ directory.

## Using the Command-Line Tool
*Quote mint only supports wsol*
```
# Buy
./target/release/raytx swap <mint> <in-amount> 0
# Sell
./target/release/raytx swap <mint> <in-amount> 1
# Sell and Close Token Account
./target/release/raytx swap <mint> <in-amount> 11
```
Replace <mint> with the address of the token you want to swap, and <amount> with the quantity you want to swap.

# Contributing
Contributions to this project are welcome. If you have any questions or suggestions, feel free to raise an issue.

# License
This project is licensed under the MIT License.
