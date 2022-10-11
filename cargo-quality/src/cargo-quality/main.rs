extern crate anyhow;
extern crate rust_code_analysis;
extern crate serde_json;

use cargo_quality::command;
use cargo_quality::log as inner_log;
use clap::Parser;

fn main() {
    inner_log::simple_logger_init();
    let cm = command::Command::parse();
    log::info!("command: {:?}", cm);

    match cm {
        // Do quality evaluation
        command::Command::Check(a) => {
            if let Err(e) = command::check::check(a) {
                panic!("Err in check. {:?}", e);
            }
        }
        // Init config file: quality-evaluation.toml
        command::Command::Init(a) => {
            if let Err(e) = command::init::init_config(a) {
                panic!("Err in init config. {:?}", e);
            }
        }
    }
}
