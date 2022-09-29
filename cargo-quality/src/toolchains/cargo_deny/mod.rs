use crate::config;
use crate::result;
use anyhow::{Context, Result};
use cargo::{core, ops, util};
use cargo_deny::{diag::Files, licenses, licenses::LicenseStore, Kid};
use krates::Builder;
use licenses::LicenseInfo;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

/// loading license store
pub(crate) fn load_license_store() -> Result<LicenseStore, anyhow::Error> {
    LicenseStore::from_cache()
}

/// License details for Crate
#[derive(Debug, Clone, Serialize, Default)]
pub struct CrateLicenseInfo {
    pub name: String,
    pub license: Vec<String>,
    pub if_has_allow_license: bool,
    pub if_has_deny_license: bool,
    pub if_has_default_license: bool,
    pub if_unlicense: bool,
}

/// Cargo-deny's license check struct
#[derive(Debug)]
pub struct LicenseCheck {
    pub project_cfg: config::ProjectInfoConfig,
    pub config: config::Config,
    pub crate_licenses: BTreeMap<Kid, Vec<String>>,
    pub license_crates: Vec<(String, Vec<Kid>)>,
    pub unlicense_crates: Vec<Kid>,
    pub result: Vec<CrateLicenseInfo>,
    pub check_result: Option<result::CheckResultDetail>,
    pub parse_result: Option<result::ParseResultDetail>,
    pub count_result: Option<result::CountResultDetail>,
}

impl super::CheckToolOption for LicenseCheck {
    /// Using cargo-deny's subcommand 'cargo deny list'.
    /// The code of getting license informations are copied and edited from
    /// cargo-deny's subcommand 'cargo deny list'
    fn check(&mut self) -> Result<()> {
        println!("LicenseCheck check: {:?}", self.project_cfg);
        let (krates, store) = rayon::join(
            || gather_krates(self.project_cfg.manifest.clone()),
            load_license_store,
        );

        let krates = krates.context("failed to gather crates")?;
        let store = store.context("failed to load license store")?;

        let gatherer = licenses::Gatherer::default()
            .with_store(std::sync::Arc::new(store))
            .with_confidence_threshold(0.8);

        let mut files = Files::new();

        let summary = gatherer.gather(&krates, &mut files, None);

        let licenses = &mut self.license_crates;
        let unlicensed = &mut self.unlicense_crates;

        for krate_lic_nfo in summary.nfos {
            let mut cur = Vec::with_capacity(2);

            match krate_lic_nfo.lic_info {
                LicenseInfo::SpdxExpression { expr, .. } => {
                    for req in expr.requirements() {
                        let s = req.req.to_string();

                        if cur.contains(&s) {
                            continue;
                        }

                        match licenses.binary_search_by(|(r, _)| r.cmp(&s)) {
                            Ok(i) => licenses[i].1.push(krate_lic_nfo.krate.id.clone()),
                            Err(i) => {
                                let mut v = Vec::with_capacity(20);
                                v.push(krate_lic_nfo.krate.id.clone());
                                licenses.insert(i, (s.clone(), v));
                            }
                        }
                        cur.push(s);
                    }
                }
                LicenseInfo::Unlicensed => {
                    unlicensed.push(krate_lic_nfo.krate.id.clone());
                }
            }
            self.crate_licenses
                .insert(krate_lic_nfo.krate.id.clone(), cur);
        }

        Ok(())
    }

    /// Parse the original license check result into the format
    /// which is suitable for users and calculation.
    fn parse(&mut self) -> Result<()> {
        println!(
            "LicenseCheck parse: crate_licenses len:{:?}",
            self.crate_licenses.len()
        );
        if let Some(config::QualityEvaluationConfig {
            license_cfg:
                Some(config::LicenseEvaluationConfig {
                    allow_licenses: Some(allow_licenses),
                    deny_licenses: Some(deny_licenses),
                    ..
                }),
            ..
        }) = &self.config.quality_evaluation_cfg
        {
            let allow_licenses: HashSet<&std::string::String> =
                HashSet::from_iter(allow_licenses.iter());
            let deny_licenses: HashSet<&std::string::String> =
                HashSet::from_iter(deny_licenses.iter());
            let mut deny_license_count: u64 = 0;
            let mut default_license_count: u64 = 0;
            for (k, v) in self.crate_licenses.iter() {
                let mut if_has_allow_license = false;
                let mut if_has_deny_license = false;
                let mut if_has_default_license = false;
                let if_unlicense = false;
                for l in v.iter() {
                    if deny_licenses.contains(l) {
                        deny_license_count += 1;
                        if_has_deny_license = true;
                    } else if allow_licenses.contains(l) {
                        if_has_allow_license = true;
                    } else {
                        default_license_count += 1;
                        if_has_default_license = true;
                    }
                }
                self.result.push(CrateLicenseInfo {
                    name: k.repr.clone(),
                    license: v.clone(),
                    if_has_allow_license,
                    if_has_deny_license,
                    if_has_default_license,
                    if_unlicense,
                });
            }
            for k in self.unlicense_crates.iter() {
                self.result.push(CrateLicenseInfo {
                    name: k.repr.clone(),
                    license: vec![],
                    if_has_allow_license: false,
                    if_has_deny_license: false,
                    if_has_default_license: true,
                    if_unlicense: true,
                });
            }
            self.parse_result = Some(result::ParseResultDetail::CargoLicense {
                deny_license: Some(deny_license_count),
                unlicense: Some(self.unlicense_crates.len() as u64),
                default_license: Some(default_license_count),
            });
        }
        self.write_result_file()?;
        Ok(())
    }

    /// Calculate the license score
    fn count(&mut self) -> Result<()> {
        println!("LicenseCheck count: {:?}", self.parse_result);
        if let Some(config::QualityEvaluationConfig {
            license_cfg:
                Some(config::LicenseEvaluationConfig {
                    deny_license_score: Some(deny_license_score),
                    default_license_score: Some(default_license_score),
                    license_eval_score: Some(license_eval_score),
                    license_eval_weight: Some(license_eval_weight),
                    ..
                }),
            ..
        }) = self.config.quality_evaluation_cfg
        {
            if let Some(result::ParseResultDetail::CargoLicense {
                deny_license: Some(deny_license),
                default_license: Some(default_license),
                ..
            }) = &self.parse_result
            {
                let score = license_eval_score
                    .saturating_sub(deny_license * deny_license_score)
                    .saturating_sub(default_license * default_license_score)
                    * 100
                    / license_eval_score;
                self.count_result = Some(result::CountResultDetail::CargoLicense {
                    score: Some(score),
                    normalized_score: Some(score * license_eval_weight / 100),
                });
            }
        }

        Ok(())
    }

    /// Process the results presented to the user
    fn result(&mut self, result: &mut result::Result) -> Result<()> {
        println!("LicenseCheck result: {:?}", self.count_result);
        if let Some(result::CountResultDetail::CargoLicense {
            score: Some(score),
            normalized_score: Some(n_score),
        }) = self.count_result
        {
            result.license_check = Some(result::LicenseCheck {
                score: Some(score),
                normalized_score: Some(n_score),
            })
        }
        Ok(())
    }
}

impl LicenseCheck {
    fn write_result_file(&self) -> Result<()> {
        let _ = File::create(&self.project_cfg.result).unwrap();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.project_cfg.result)
            .unwrap();
        for v in self.result.iter() {
            file.write_all(toml::to_string(v).unwrap().as_bytes())?;
            file.write_all("\n".as_bytes())?;
        }
        Ok(())
    }
}

// fixme: copy and edit from cargo-deny.
struct MetadataOptions {
    no_default_features: bool,
    all_features: bool,
    features: Vec<String>,
    manifest_path: PathBuf,
    frozen: bool,
    locked: bool,
    offline: bool,
}

// fixme: copy and edit from cargo-deny.
fn get_metadata(opts: MetadataOptions) -> Result<krates::cm::Metadata, anyhow::Error> {
    let mut config = util::Config::default()?;

    config.configure(
        0,
        true,
        None,
        opts.frozen,
        opts.locked,
        opts.offline,
        &None,
        &[],
        &[],
    )?;

    let mut manifest_path = opts.manifest_path;

    if !manifest_path.is_absolute() {
        manifest_path = std::env::current_dir()
            .context("unable to determine current directory")?
            .join(manifest_path);
    }

    let features = std::rc::Rc::new(
        opts.features
            .into_iter()
            .map(|feat| core::FeatureValue::new(util::interning::InternedString::new(&feat)))
            .collect(),
    );

    let ws = core::Workspace::new(&manifest_path, &config)?;
    let options = ops::OutputMetadataOptions {
        cli_features: core::resolver::features::CliFeatures {
            features,
            all_features: opts.all_features,
            uses_default_features: !opts.no_default_features,
        },
        no_deps: false,
        version: 1,
        filter_platforms: vec![],
    };

    let md = ops::output_metadata(&ws, &options)?;
    let md_value = serde_json::to_value(md)?;

    Ok(serde_json::from_value(md_value)?)
}

// fixme: copy and edit from cargo-deny.
pub fn gather_krates(manifest: PathBuf) -> Result<cargo_deny::Krates, anyhow::Error> {
    let start = std::time::Instant::now();
    let metadata = get_metadata(MetadataOptions {
        no_default_features: true,
        all_features: true,
        features: vec![],
        manifest_path: manifest,
        frozen: false,
        locked: false,
        offline: false,
    })?;

    let gb = Builder::new();
    let graph = gb.build_with_metadata(metadata, |filtered: krates::cm::Package| {
        match filtered.source {
            Some(src) => {
                if src.is_crates_io() {
                    log::debug!("filtered {} {}", filtered.name, filtered.version);
                } else {
                    log::debug!("filtered {} {} {}", filtered.name, filtered.version, src);
                }
            }
            None => log::debug!("filtered {} {}", filtered.name, filtered.version),
        }
    });

    if let Ok(ref krates) = graph {
        let end = std::time::Instant::now();
        log::info!(
            "gathered {} crates in {}ms",
            krates.len(),
            (end - start).as_millis()
        );
    }

    Ok(graph?)
}
