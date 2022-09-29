pub mod cargo_deny;
pub mod clippy;
pub mod rust_code_analysis;

use crate::config;
use crate::result;
use anyhow::Result;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

pub enum CheckTool {
    ClippyForStaticCheck,
    CargoDenyForLicenseCheck,
    RustCodeAnalysisForMeasure,
}

/// Trait for integrating tools.
pub trait CheckToolOption: std::fmt::Debug {
    /// 集成工具的检查部分
    fn check(&mut self) -> Result<()>;
    /// 解析工具产生的原始结果
    fn parse(&mut self) -> Result<()>;
    /// 根据配置，计算检查项的得分
    fn count(&mut self) -> Result<()>;
    /// 对检查项结果进行最终处理，呈现给用户
    fn result(&mut self, result: &mut result::Result) -> Result<()>;
}

/// Create the specific tool check item.
pub fn make_check(
    cfg: config::Config,
    tool: CheckTool,
    project_cfg: &config::ProjectInfoConfig,
) -> Box<dyn CheckToolOption> {
    let project = config::ProjectInfoConfig {
        dir: project_cfg.dir.clone(),
        manifest: project_cfg.manifest.clone(),
        result: make_result_path(&tool, &project_cfg.result),
    };
    match tool {
        CheckTool::ClippyForStaticCheck => Box::new(clippy::Clippy {
            project_cfg: project,
            config: cfg,
            lint_info: HashMap::new(),
            check_result: None,
            parse_result: None,
            count_result: None,
        }),
        CheckTool::CargoDenyForLicenseCheck => Box::new(cargo_deny::LicenseCheck {
            project_cfg: project,
            config: cfg,
            crate_licenses: BTreeMap::new(),
            license_crates: vec![],
            unlicense_crates: vec![],
            result: vec![],
            check_result: None,
            parse_result: None,
            count_result: None,
        }),
        CheckTool::RustCodeAnalysisForMeasure => Box::new(rust_code_analysis::RustCodeAnalysis {
            project_cfg: project,
            config: cfg,
            dir_list: HashMap::new(),
            func_info: vec![],
            check_result: None,
            parse_result: None,
            count_result: None,
        }),
    }
}

/// Store the check results.
/// The main purpose of this file is to make it easy for users to view detailed problems and fix them.
#[allow(clippy::ptr_arg)]
fn make_result_path(c: &CheckTool, dir: &PathBuf) -> PathBuf {
    let mut result_path = dir.clone();
    match c {
        CheckTool::ClippyForStaticCheck => result_path.push(r"static_check.txt"),
        CheckTool::CargoDenyForLicenseCheck => result_path.push(r"license_check.txt"),
        CheckTool::RustCodeAnalysisForMeasure => result_path.push(r"measure_check.txt"),
    }
    result_path
}
