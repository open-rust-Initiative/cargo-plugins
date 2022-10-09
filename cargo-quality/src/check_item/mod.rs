use crate::config;
use crate::result;
use anyhow::Result;
use std::path::PathBuf;
pub mod license_check;
pub mod measure_check;
pub mod static_check;

/// Trait for check item
pub trait CheckItem: std::fmt::Debug {
    fn check(&mut self, result: &mut result::Result) -> Result<()>;
}

/// Create check item
pub fn make_check<'b>(
    c: config::Config,
    q: &'b config::QualityEvaluation,
    p: &'b config::ProjectInfoConfig,
) -> Box<dyn CheckItem> {
    let project_cfg = config::ProjectInfoConfig {
        dir: p.dir.clone(),
        manifest: p.manifest.clone(),
        result: make_result_path(q, &p.result),
    };
    match &q {
        config::QualityEvaluation::StaticCheck => Box::new(static_check::StaticCheck {
            project: project_cfg,
            config: c,
        }),
        config::QualityEvaluation::License => Box::new(license_check::LicenseCheck {
            project: project_cfg,
            config: c,
        }),
        config::QualityEvaluation::Measure => Box::new(measure_check::MeasureCheck {
            project: project_cfg,
            config: c,
        }),
    }
}

/// Store the check item results
/// The purpose of this file is to make it easy for users to view detailed problems and fix them
#[allow(clippy::ptr_arg)]
fn make_result_path(c: &config::QualityEvaluation, dir: &PathBuf) -> PathBuf {
    let mut result_path = dir.clone();
    match c {
        config::QualityEvaluation::StaticCheck => result_path.push(r"static_check"),
        config::QualityEvaluation::License => result_path.push(r"license_check"),
        config::QualityEvaluation::Measure => result_path.push(r"measure_check"),
    }
    result_path
}
