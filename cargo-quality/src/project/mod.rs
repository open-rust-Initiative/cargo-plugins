use crate::check_item;
use crate::config;
use crate::result;
use anyhow::{Context, Result};
use std::path::PathBuf;

#[derive(Debug)]
pub struct Project {
    pub manifest: PathBuf,
    pub result_path: PathBuf,
    pub cfg: config::Config,
    pub check_item: Vec<Box<dyn check_item::CheckItem>>,
    pub result: result::Result,
}

impl Project {
    pub fn new(project_path: PathBuf, config: &config::Config) -> Result<Project> {
        let check_info: Vec<config::QualityEvaluation> = if let Some(v) = &config.check_quality_item
        {
            v.to_vec()
        } else {
            config::QualityEvaluation::get_all()
        };
        let mut manifest = project_path.clone();
        manifest.push("Cargo.toml");

        let mut result_dir_path = PathBuf::new();
        result_dir_path.push(
            std::env::current_dir()
                .with_context(|| format!("Find current path failed! {:?}", result_dir_path))?,
        );
        result_dir_path.push(r"cargo_quality_evaluation");
        std::fs::create_dir_all(&result_dir_path)
            .with_context(|| format!("Create result dir path failed! {:?}", result_dir_path))?;

        let project_cfg = config::ProjectInfoConfig {
            dir: project_path.clone(),
            manifest: manifest.clone(),
            result: result_dir_path.clone(),
        };
        let mut pr = Project {
            manifest: project_path,
            result_path: result_dir_path.clone(),
            cfg: config.clone(),
            check_item: vec![],
            result: Default::default(),
        };
        for q in check_info {
            pr.check_item
                .push(check_item::make_check(config.clone(), &q, &project_cfg));
        }

        Ok(pr)
    }

    // fixme: Use multi-threaded to executing task
    pub fn execute(&mut self) -> Result<()> {
        for i in &mut self.check_item {
            i.check(&mut self.result)?;
        }
        Ok(())
    }
    pub fn get_result(&self) -> result::Result {
        self.result.clone()
    }
}
