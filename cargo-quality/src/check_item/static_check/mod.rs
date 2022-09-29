use crate::config;
use crate::result;
use crate::toolchains;
use anyhow::{Context, Result};

#[derive(Debug)]
pub struct StaticCheckResult {
    score: Option<u64>,
    normalized_score: Option<u64>,
}

impl result::ResultInfo for StaticCheckResult {
    fn score(&self) -> Option<u64> {
        return self.score;
    }

    fn normalized_score(&self) -> Option<u64> {
        return self.normalized_score;
    }

    fn details(&self) -> Option<String> {
        unimplemented!();
    }
}

/// Static check struct
#[derive(Debug)]
pub struct StaticCheck {
    pub project: config::ProjectInfoConfig,
    pub config: config::Config,
}

impl super::CheckItem for StaticCheck {
    /// Perform the Static check
    /// The detail result is written to file
    fn check(&mut self, result: &mut result::Result) -> Result<()> {
        println!("Static check: {:?}", self.project);
        std::fs::create_dir_all(&self.project.result)
            .with_context(|| format!("Create result dir path failed! {:?}", self.project))?;
        let mut static_check_tool = toolchains::make_check(
            self.config.clone(),
            toolchains::CheckTool::ClippyForStaticCheck,
            &self.project,
        );
        static_check_tool.check()?;
        static_check_tool.parse()?;
        static_check_tool.count()?;
        static_check_tool.result(result)?;
        Ok(())
    }
}
