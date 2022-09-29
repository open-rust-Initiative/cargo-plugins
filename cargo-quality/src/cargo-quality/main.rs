extern crate anyhow;
extern crate rust_code_analysis;
extern crate serde_json;

use cargo_guard::command;
use clap::Parser;

fn main() {
    let cm = command::Command::parse();
    println!("command: {:?}", cm);

    match cm {
        // Do quality evaluation
        command::Command::Check(a) => {
            if let Err(e) = command::check::check(a) {
                panic!("Err in check. {:?}", e);
            }
            return;
        }
        // Init config file: quality-evaluation.toml
        command::Command::Init(a) => {
            if let Err(e) = command::init::init_config(a) {
                panic!("Err in init config. {:?}", e);
            }
            return;
        }
    }
}
