use anyhow::{Context, Error, Result};
use serde_derive::Deserialize;
use std::default::Default;
use std::path::PathBuf;

#[derive(clap::ValueEnum, Debug, PartialEq, Eq, Copy, Clone, Deserialize)]
pub enum CommunityEcology {
    DeveloperCount,
}

impl CommunityEcology {
    pub fn get_all() -> Vec<CommunityEcology> {
        vec![CommunityEcology::DeveloperCount]
    }
}

impl std::str::FromStr for CommunityEcology {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let l = s.to_ascii_lowercase();

        Ok(match l.as_str() {
            "developer_count" => Self::DeveloperCount,
            _ => anyhow::bail!("unkonw CommunityEcology option"),
        })
    }
}

#[derive(clap::ValueEnum, Debug, PartialEq, Eq, Copy, Clone, Deserialize)]
pub enum QualityEvaluation {
    StaticCheck,
    License,
    Measure,
}

impl QualityEvaluation {
    pub fn get_all() -> Vec<QualityEvaluation> {
        vec![
            QualityEvaluation::StaticCheck,
            QualityEvaluation::License,
            QualityEvaluation::Measure,
        ]
    }
}

impl std::str::FromStr for QualityEvaluation {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_ascii_lowercase().as_str() {
            "static_check" => Self::StaticCheck,
            "license" => Self::License,
            "measure" => Self::Measure,
            _ => anyhow::bail!("unkonw QualityEvaluation option"),
        })
    }
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Config {
    pub check_quality_item: Option<Vec<QualityEvaluation>>,
    pub exclude_dir: Option<Vec<String>>,
    pub check_community_ecology_item: Option<Vec<CommunityEcology>>,
    pub community_ecology_cfg: Option<CommunityEcologyConfig>,
    pub quality_evaluation_cfg: Option<QualityEvaluationConfig>,
    pub project_info_config: Option<ProjectInfoConfig>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct CommunityEcologyConfig {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct QualityEvaluationConfig {
    pub static_check_cfg: Option<StaticCheckEvaluationConfig>,
    pub measeure_cfg: Option<MeasureEvaluationConfig>,
    pub license_cfg: Option<LicenseEvaluationConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StaticCheckEvaluationConfig {
    pub error_score: Option<u64>,
    pub warn_score: Option<u64>,
    pub static_check_score: Option<u64>,
    pub static_check_weight: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MeasureEvaluationConfig {
    pub large_cyclomatic_complexity: Option<u64>,
    pub large_cyclomatic_complexity_score: Option<u64>,
    pub large_cognitive_complexity: Option<u64>,
    pub large_cognitive_complexity_score: Option<u64>,
    pub large_num_rows_function: Option<u64>,
    pub large_num_rows_function_score: Option<u64>,
    pub large_num_rows_file: Option<u64>,
    pub large_num_rows_file_score: Option<u64>,
    pub measure_score: Option<u64>,
    pub measure_weight: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LicenseEvaluationConfig {
    pub allow_licenses: Option<Vec<String>>,
    pub deny_licenses: Option<Vec<String>>,
    pub deny_license_score: Option<u64>,
    pub default_license_score: Option<u64>,
    pub unlicense_score: Option<u64>,
    pub license_eval_score: Option<u64>,
    pub license_eval_weight: Option<u64>,
}

pub fn parse(path: PathBuf) -> Result<Config> {
    let content = std::fs::read(&path)
        .with_context(|| format!("Failed to read config file from {:?}", path))?;
    toml::from_str(std::str::from_utf8(&content).unwrap()).map_err(anyhow::Error::from)
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct ProjectInfoConfig {
    pub manifest: PathBuf,
    pub dir: PathBuf,
    pub result: PathBuf,
}
