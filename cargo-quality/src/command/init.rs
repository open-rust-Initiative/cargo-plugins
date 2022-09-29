use std::path::PathBuf;

use anyhow::{ensure, Context, Error};

#[derive(clap::Parser, Debug)]
pub struct Args {
    #[clap(short, long, action)]
    pub config: Option<PathBuf>,
    #[clap(short, long, action)]
    pub project: Option<PathBuf>,
}

const CONFIG: &[u8] = include_bytes!("../../quality-evaluation-template.toml");

/// Subcommand init config
#[allow(clippy::redundant_clone)]
pub fn init_config(args: Args) -> Result<(), Error> {
    let cfg_path: PathBuf = if let Some(a) = args.project {
        let mut t = a.clone();
        t.push("quality-evaluation.toml");
        t
    } else {
        PathBuf::from("quality-evaluation.toml")
    };

    ensure!(
        std::fs::metadata(&cfg_path).is_err(),
        "Unable to init config: '{}' already exists",
        cfg_path.display()
    );

    ensure!(
        cfg_path.file_name().is_some(),
        "Unable to init config: '{}' has an invalid filename",
        cfg_path.display()
    );

    std::fs::write(&cfg_path, CONFIG).context("unable to write config file")?;

    Ok(())
}
