use crate::config;
use crate::project;
use anyhow::Error;
use std::path::PathBuf;

#[derive(clap::ValueEnum, Debug, PartialEq, Eq, Copy, Clone)]
pub enum CheckItem {
    CommunityEcology,
    QualityEvaluation,
    All,
}

#[derive(clap::Parser, Debug)]
pub struct Args {
    #[clap(short, long, action)]
    pub config: Option<PathBuf>,
    #[clap(short, long, action)]
    pub project: Option<PathBuf>,
    /// The check to perform
    #[clap(value_enum, action, default_value_t = CheckItem::All)]
    pub check_item: CheckItem,
}

/// Subcommand check
pub fn check(args: Args) -> Result<(), Error> {
    let now_path = std::env::current_dir().unwrap();

    let cfg_path = match args.config {
        Some(p) => p,
        _ => {
            let mut p = PathBuf::new();
            p.push(now_path.clone());
            p.push(r"quality-evaluation.toml");
            p
        }
    };
    let cfg = config::parse(cfg_path)?;
    println!("config : {:?}", cfg);

    let project_path = match args.project {
        Some(p) => {
            println!("project_path from arg: {:?}", p);
            p
        }
        _ => {
            println!("project_path default {:?}", now_path);
            now_path
        }
    };

    if let Ok(mut p) = project::Project::new(project_path, &cfg) {
        p.execute()?;
        let r = p.get_result();
        println!("result json: {:?}", serde_json::json!(r));
    }

    Ok(())
}
