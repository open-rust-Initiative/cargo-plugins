use crate::config;
use crate::result;
use crate::util;
use anyhow::Result;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

/// Clippy check struct
#[derive(Debug)]
pub struct Clippy {
    pub project_cfg: config::ProjectInfoConfig,
    pub config: config::Config,
    pub lint_info: HashMap<u32, LintInfo>,
    pub check_result: Option<result::CheckResultDetail>,
    pub parse_result: Option<result::ParseResultDetail>,
    pub count_result: Option<result::CountResultDetail>,
}

/// Lint kind for Clippy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintKind {
    ClippyWarn(String),
    ClippyDeny(String),
    RustWarn(String),
    RustDeny(String),
    None,
}

/// Information of lint
#[derive(Debug, Clone)]
pub struct LintInfo {
    pub ind: u32,
    pub lint_kind: LintKind,
    pub content: Vec<String>,
}

impl Clippy {
    /// Save lint item
    fn save_lint_item(&mut self, ind: u32, lint_kind: LintKind, content: Vec<String>) {
        if lint_kind == LintKind::None {
            return;
        }
        let lint = LintInfo {
            ind,
            lint_kind,
            content,
        };
        self.lint_info.insert(ind, lint);
    }
}

impl super::CheckToolOption for Clippy {
    /// Perform the Clippy check
    /// The result is written to file
    fn check(&mut self) -> Result<()> {
        println!("Clippy check: {:?}", self.project_cfg);
        if let Err(e) = util::cargo(
            "clippy",
            &self.project_cfg.manifest,
            &self.project_cfg.result,
            util::StdOutput::Err,
        ) {
            println!("Clippy check failed: {:?}", e);
        }
        self.check_result = Some(result::CheckResultDetail::Clippy {
            result: Some(self.project_cfg.result.clone()),
        });
        Ok(())
    }

    /// Read and parse the execution result of Clippy from the file
    /// Parse and classify Lint
    fn parse(&mut self) -> Result<()> {
        println!("Clippy parse: {:?}", self.check_result);
        if let Some(result::CheckResultDetail::Clippy { result: Some(p) }) = &self.check_result {
            let f = File::open(&p)?;
            let buf = BufReader::new(f).lines();
            let mut block_ind: u32 = 1;
            let mut lint_kind = LintKind::None;
            let mut content = vec![];
            // Clippy's result text is processed line by line
            for l in buf.flatten() {
                // new lint block
                if l.is_empty() {
                    self.save_lint_item(block_ind, lint_kind.clone(), content.clone());
                    content.clear();
                    block_ind += 1;
                }
                // Find the Lint type
                if l.contains(r"#[warn(")
                    || l.contains(r"#[deny(")
                    || l.contains(r"#[warn(clippy::")
                    || l.contains(r"#[deny(clippy::")
                {
                    lint_kind = get_lint_kind(&l);
                }
                // Not a new lint block, keep saving rows
                if !l.is_empty() {
                    content.push(l.clone());
                }
            }
        }

        let mut res_err: u64 = 0;
        let mut res_warn: u64 = 0;
        for (_, v) in self.lint_info.iter() {
            match v.lint_kind {
                LintKind::ClippyWarn(_) | LintKind::RustWarn(_) => {
                    res_warn += 1;
                }
                LintKind::ClippyDeny(_) | LintKind::RustDeny(_) => {
                    res_err += 1;
                }
                _ => {}
            }
        }
        self.parse_result = Some(result::ParseResultDetail::Clippy {
            error: Some(res_err),
            warn: Some(res_warn),
        });
        Ok(())
    }

    /// Calculate the execution result of Clippy according to the config
    fn count(&mut self) -> Result<()> {
        println!("Clippy count: {:?}", self.parse_result);
        if let Some(config::QualityEvaluationConfig {
            static_check_cfg: Some(static_check_cfg),
            ..
        }) = &self.config.quality_evaluation_cfg
        {
            if let Some(result::ParseResultDetail::Clippy {
                warn: Some(warn_count),
                error: Some(err_count),
            }) = &self.parse_result
            {
                if let config::StaticCheckEvaluationConfig {
                    static_check_score: Some(static_check_score),
                    warn_score: Some(warn_score),
                    error_score: Some(error_score),
                    static_check_weight: Some(static_check_weight),
                } = static_check_cfg
                {
                    let score = static_check_score
                        .saturating_sub(warn_count * warn_score)
                        .saturating_sub(err_count * error_score)
                        * 100
                        / static_check_score;
                    let normalized_score = score * static_check_weight / 100;
                    self.count_result = Some(result::CountResultDetail::Clippy {
                        score: Some(score),
                        normalized_score: Some(normalized_score),
                    });
                }
            }
        }
        Ok(())
    }

    /// Process the results presented to the user
    /// Clippy is currently the only static checking tool
    fn result(&mut self, result: &mut result::Result) -> Result<()> {
        println!("Clippy result: {:?}", self.count_result);
        if let Some(result::CountResultDetail::Clippy {
            score: Some(score),
            normalized_score: Some(n_score),
        }) = self.count_result
        {
            result.static_check = Some(result::StaticCheck {
                score: Some(score),
                normalized_score: Some(n_score),
            })
        }
        println!("clippy result: {:?}", result);
        Ok(())
    }
}

/// Parse the string to get Lintkind
fn get_lint_kind(s: &str) -> LintKind {
    let mut lint = LintKind::None;
    if let Some(p) = s.find(r"#[warn(") {
        if let Some(e) = s.find(')') {
            lint = LintKind::RustWarn(s[p..e].to_string());
        }
    }
    if let Some(p) = s.find("r#[deny(") {
        if let Some(e) = s.find(')') {
            lint = LintKind::RustDeny(s[p..e].to_string());
        }
    }
    if let Some(p) = s.find("r#[warn(clippy::") {
        if let Some(e) = s.find(')') {
            lint = LintKind::ClippyWarn(s[p..e].to_string());
        }
    }
    if let Some(p) = s.find("r#[deny(clippy::") {
        if let Some(e) = s.find(')') {
            lint = LintKind::ClippyDeny(s[p..e].to_string());
        }
    }
    lint
}
