use crate::config;
use crate::result;
use crate::toolchains;
use anyhow::{Context, Result};

#[derive(Debug)]
pub struct MeasureCheckResult {
    score: Option<u64>,
    normalized_score: Option<u64>,
}

impl result::ResultInfo for MeasureCheckResult {
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

/// Measure check struct
#[derive(Debug)]
pub struct MeasureCheck {
    pub project: config::ProjectInfoConfig,
    pub config: config::Config,
}

impl super::CheckItem for MeasureCheck {
    /// Perform the Measure check
    /// The result is written to file
    fn check(&mut self, result: &mut result::Result) -> Result<()> {
        println!("Measure check: {:?}", self.project);
        std::fs::create_dir_all(&self.project.result)
            .with_context(|| format!("Create result dir path failed! {:?}", self.project))?;
        let mut measure_check_tool = toolchains::make_check(
            self.config.clone(),
            toolchains::CheckTool::RustCodeAnalysisForMeasure,
            &self.project,
        );
        measure_check_tool.check()?;
        measure_check_tool.parse()?;
        measure_check_tool.count()?;
        measure_check_tool.result(result)?;
        Ok(())
    }
}
