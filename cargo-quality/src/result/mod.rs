use serde_derive::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum CheckResultDetail {
    Clippy { result: Option<PathBuf> },
    CodeAnalysis { result: Option<PathBuf> },
    CargoLicense { result: Option<PathBuf> },
}

#[derive(Debug, Clone)]
pub enum ParseResultDetail {
    Clippy {
        error: Option<u64>,
        warn: Option<u64>,
    },
    CodeAnalysis {
        large_cyclomatic_complexity: Option<u64>,
        large_cognitive_complexity: Option<u64>,
        large_num_rows_file: Option<u64>,
        large_num_rows_function: Option<u64>,
    },
    CargoLicense {
        deny_license: Option<u64>,
        unlicense: Option<u64>,
        default_license: Option<u64>,
    },
}

#[derive(Debug, Clone)]
pub enum CountResultDetail {
    Clippy {
        score: Option<u64>,
        normalized_score: Option<u64>,
    },
    CodeAnalysis {
        score: Option<u64>,
        normalized_score: Option<u64>,
    },
    CargoLicense {
        score: Option<u64>,
        normalized_score: Option<u64>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct StaticCheck {
    pub score: Option<u64>,
    pub normalized_score: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CodeMeasure {
    pub score: Option<u64>,
    pub normalized_score: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CodeDuplication {
    pub score: Option<u64>,
    pub normalized_score: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DependentCrate {
    pub score: Option<u64>,
    pub normalized_score: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct SecurityCheck {
    pub score: Option<u64>,
    pub normalized_score: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DynamicCheck {
    pub score: Option<u64>,
    pub normalized_score: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct VulnerabilityScan {
    pub score: Option<u64>,
    pub normalized_score: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct LicenseCheck {
    pub score: Option<u64>,
    pub normalized_score: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CompileBuildInfoCheck {
    pub score: Option<u64>,
    pub normalized_score: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct DocCheck {
    pub score: Option<u64>,
    pub normalized_score: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TestCheck {
    pub score: Option<u64>,
    pub normalized_score: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ArchitectureDesignCheck {
    pub score: Option<u64>,
    pub normalized_score: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Result {
    pub static_check: Option<StaticCheck>,
    pub code_measure: Option<CodeMeasure>,
    pub code_duplication: Option<CodeDuplication>,
    pub dependent_crate: Option<DependentCrate>,
    pub security_check: Option<SecurityCheck>,
    pub dynamic_check: Option<DynamicCheck>,
    pub vulnerability_scan: Option<VulnerabilityScan>,
    pub license_check: Option<LicenseCheck>,
    pub compile_build_info_check: Option<CompileBuildInfoCheck>,
    pub doc_check: Option<DocCheck>,
    pub test_check: Option<TestCheck>,
    pub architecture_design_check: Option<ArchitectureDesignCheck>,
}

pub trait ResultInfo {
    fn score(&self) -> Option<u64>;
    fn normalized_score(&self) -> Option<u64>;
    fn details(&self) -> Option<String>;
}
