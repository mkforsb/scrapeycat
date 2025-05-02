# Installation

## Linux/Unix
### Using Cargo

The recommended way to install Scrapeycat is to use Cargo, the Rust package manager. As an example,
here are the steps required to install Scrapeycat on a fresh copy of Ubuntu 24.04:

```sh
# 1. Install necessary tools and dependencies
$ sudo apt install curl build-essential pkg-config libssl-dev

# 2. Install Rust and Cargo according to the instructions on https://rustup.rs
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# ** restart terminal **

# 3. Install Scrapeycat using Cargo
$ cargo install --git https://github.com/mkforsb/scrapeycat
```

Installation on other systems may require a different step 1, but should otherwise be very
similar.

## Windows
At this time there is no Windows version of Scrapeycat, but you should be able to run the Linux
version using the steps for Linux shown above under
[WSL](https://learn.microsoft.com/en-us/windows/wsl/install).
