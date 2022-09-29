pub mod check;
pub mod init;

use clap::Parser;

#[derive(Parser, Debug)]
pub enum Command {
    /// Check project.
    #[clap(name = "check")]
    Check(check::Args),
    /// Init config file: quality-evaluation.toml for project.
    #[clap(name = "init")]
    Init(init::Args),
}
