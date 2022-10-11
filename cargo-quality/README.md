### Cargo Quality

#### Why do we need this?

There are many crates in the Rust ecosystem, and it is hard to find the right crate for your project. This is why we created Cargo Quality, a tool to evaluate the quality of Rust crate.

#### Install

1. You need to compile the project first.
```shell
cargo build
```

2. Put the executable file `cargo-quality` in linux or `cargo-quality.exe` in windows to your #CARGO_HOME/bin

3.Enter the item you want to detect through the command line.

```shell
cd ./lathes
````

4. Init config.
```shell
cargo-quality init
````

5. Do check.
```shell
cargo-quality check
````