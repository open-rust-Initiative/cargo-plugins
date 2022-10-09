use crate::config;
use crate::result;
use crate::util;

use anyhow::{Context, Result};
use clap::Parser;
use globset::{Glob, GlobSet, GlobSetBuilder};
use log;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{hash_map, HashMap};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use std::process;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread::available_parallelism;
use walkdir::DirEntry;

extern crate lazy_static;
use lazy_static::lazy_static;
use rust_code_analysis::ParserTrait;
use rust_code_analysis::{
    action, get_from_ext, get_function_spaces, get_ops, guess_language, preprocess, read_file,
    read_file_with_eol,
};
use rust_code_analysis::{
    CommentRm, CommentRmCfg, ConcurrentRunner, Count, CountCfg, Dump, DumpCfg, FilesData, Find,
    FindCfg, Function, FunctionCfg, Metrics, MetricsCfg, OpsCfg, OpsCode, PreprocParser,
    PreprocResults, SpaceKind,
};
use rust_code_analysis::{FuncSpace, LANG};

lazy_static! {
    static ref FUNC_SPACE_RESULT: Mutex<Vec<FuncInfo>> = Mutex::new(vec![]);
}

// fixme: copy and edit from rust-code-analysis
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FuncKind {
    /// An unknown space
    #[default]
    Unknown,
    /// A function space
    Function,
    /// A class space
    Class,
    /// A struct space
    Struct,
    /// A `Rust` trait space
    Trait,
    /// A `Rust` implementation space
    Impl,
    /// A general space
    Unit,
    /// A `C/C++` namespace
    Namespace,
    /// An interface
    Interface,
}

// fixme: copy and edit from rust-code-analysis
fn get_space_kind_name(s: &SpaceKind) -> &str {
    match s {
        SpaceKind::Unknown => "unknown",
        SpaceKind::Function => "function",
        SpaceKind::Class => "class",
        SpaceKind::Struct => "struct",
        SpaceKind::Trait => "trait",
        SpaceKind::Impl => "impl",
        SpaceKind::Unit => "unit",
        SpaceKind::Namespace => "namespace",
        SpaceKind::Interface => "interface",
    }
}

/// Store the parsing result information of each function and file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FuncInfo {
    pub name: String,
    pub path: String,
    pub start_line: usize,
    pub end_line: usize,
    pub if_large_file: bool,
    pub if_large_function: bool,
    pub kind: FuncKind,
    pub cognitive: usize,
    pub if_large_cognitive: bool,
    pub cyclomatic: usize,
    pub if_large_cyclomatic: bool,
}

/// Save the results of rust-code-analysis in thread-local variables.
fn save_funcspace(path: String, f: &FuncSpace) {
    if f.kind == SpaceKind::Unit {
        if let Some(n) = &f.name {
            let s = FuncInfo {
                path: n.clone(),
                name: n.clone(),
                start_line: f.start_line,
                end_line: f.end_line,
                kind: FuncKind::Unit,
                cognitive: 0,
                cyclomatic: 0,
                ..Default::default()
            };
            FUNC_SPACE_RESULT.lock().unwrap().push(s);
        }
    } else if f.kind == SpaceKind::Function {
        if let Some(n) = &f.name {
            let s = FuncInfo {
                path: path.clone().as_mut_str().to_owned()
                    + "&&&FuncName:"
                    + n.as_str()
                    + "&&&Kind:"
                    + get_space_kind_name(&f.kind),
                name: n.clone(),
                start_line: f.start_line,
                end_line: f.end_line,
                kind: FuncKind::Function,
                cognitive: f.metrics.cognitive.cognitive() as usize,
                cyclomatic: f.metrics.cyclomatic.cyclomatic() as usize,
                ..Default::default()
            };
            FUNC_SPACE_RESULT.lock().unwrap().push(s);
        }
    }
    if !f.spaces.is_empty() {
        for i in f.spaces.iter() {
            let p: String = if path.is_empty() {
                f.name.as_ref().unwrap().clone()
            } else {
                path.clone()
                    + "&&&Name:"
                    + f.name
                        .as_ref()
                        .unwrap_or(&"parse_func_name_failed".to_string())
                        .as_str()
                    + "&&&Kind:"
                    + get_space_kind_name(&f.kind)
            };
            save_funcspace(p, i);
        }
    }
}

// fixme: copy and edit from rust-code-analysis
#[derive(Debug, Clone)]
pub enum Format {
    Cbor,
    Json,
    Toml,
    Yaml,
}

// fixme: copy and edit from rust-code-analysis
impl Format {
    pub fn all() -> &'static [&'static str] {
        &["cbor", "json", "toml", "yaml"]
    }

    pub fn dump_formats<T: Serialize>(
        &self,
        space: &T,
        path: &Path,
        output_path: &Option<PathBuf>,
        pretty: bool,
    ) -> std::io::Result<()> {
        if output_path.is_none() {
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();

            match self {
                Format::Cbor => Err(Error::new(
                    ErrorKind::Other,
                    "Cbor format cannot be printed to stdout",
                )),
                Format::Json => {
                    let json_data = if pretty {
                        serde_json::to_string_pretty(&space).unwrap()
                    } else {
                        serde_json::to_string(&space).unwrap()
                    };
                    writeln!(stdout, "{}", json_data)
                }
                Format::Toml => {
                    let toml_data = if pretty {
                        toml::to_string_pretty(&space).unwrap()
                    } else {
                        toml::to_string(&space).unwrap()
                    };
                    writeln!(stdout, "{}", toml_data)
                }
                Format::Yaml => writeln!(stdout, "{}", serde_yaml::to_string(&space).unwrap()),
            }
        } else {
            let format_ext = match self {
                Format::Cbor => ".cbor",
                Format::Json => ".json",
                Format::Toml => ".toml",
                Format::Yaml => ".yml",
            };

            // Remove root /
            let path = path.strip_prefix("/").unwrap_or(path);

            // Remove root ./
            let path = path.strip_prefix("./").unwrap_or(path);

            // Replace .. with . to keep files inside the output folder
            let cleaned_path: Vec<&str> = path
                .iter()
                .map(|os_str| {
                    let s_str = os_str.to_str().unwrap();
                    if s_str == ".." {
                        "."
                    } else {
                        s_str
                    }
                })
                .collect();
            let mut path_name = "file_result___".to_string();
            for str in cleaned_path.iter() {
                //log::info!("output path: str: {:?}", str);
                if !str.contains('\\') && !str.contains('/') && !str.contains(':') {
                    path_name += str;
                    path_name += "___";
                }
            }
            path_name = path_name[0..path_name.len() - 1].to_string() + format_ext;

            let mut format_file =
                File::create(output_path.as_ref().unwrap().join(path_name)).unwrap();
            match self {
                Format::Cbor => serde_cbor::to_writer(format_file, &space)
                    .map_err(|e| Error::new(ErrorKind::Other, e.to_string())),
                Format::Json => {
                    if pretty {
                        serde_json::to_writer_pretty(format_file, &space)
                            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))
                    } else {
                        serde_json::to_writer(format_file, &space)
                            .map_err(|e| Error::new(ErrorKind::Other, e.to_string()))
                    }
                }
                Format::Toml => {
                    let toml_data = if pretty {
                        toml::to_string_pretty(&space).unwrap()
                    } else {
                        toml::to_string(&space).unwrap()
                    };
                    format_file.write_all(toml_data.as_bytes())
                }
                Format::Yaml => serde_yaml::to_writer(format_file, &space)
                    .map_err(|e| Error::new(ErrorKind::Other, e.to_string())),
            }
        }
    }
}

// fixme: copy and edit from rust-code-analysis
impl FromStr for Format {
    type Err = String;

    fn from_str(format: &str) -> Result<Self, Self::Err> {
        match format {
            "cbor" => Ok(Format::Cbor),
            "json" => Ok(Format::Json),
            "toml" => Ok(Format::Toml),
            "yaml" => Ok(Format::Yaml),
            format => Err(format!("{:?} is not a supported format", format)),
        }
    }
}

// fixme: copy and edit from rust-code-analysis
#[derive(Debug)]
struct Config {
    dump: bool,
    in_place: bool,
    comments: bool,
    find_filter: Vec<String>,
    count_filter: Vec<String>,
    language: Option<LANG>,
    function: bool,
    metrics: bool,
    ops: bool,
    output_format: Option<Format>,
    output: Option<PathBuf>,
    pretty: bool,
    line_start: Option<usize>,
    line_end: Option<usize>,
    preproc_lock: Option<Arc<Mutex<PreprocResults>>>,
    preproc: Option<Arc<PreprocResults>>,
    count_lock: Option<Arc<Mutex<Count>>>,
}

// fixme: copy and edit from rust-code-analysis
fn mk_globset(elems: Vec<String>) -> GlobSet {
    if elems.is_empty() {
        return GlobSet::empty();
    }

    let mut globset = GlobSetBuilder::new();
    elems.iter().filter(|e| !e.is_empty()).for_each(|e| {
        if let Ok(glob) = Glob::new(e) {
            globset.add(glob);
        }
    });
    globset.build().map_or(GlobSet::empty(), |globset| globset)
}

// fixme: copy and edit from rust-code-analysis
fn act_on_file(path: PathBuf, cfg: &Config) -> std::io::Result<()> {
    let source = if let Some(source) = read_file_with_eol(&path)? {
        source
    } else {
        return Ok(());
    };

    let language = if let Some(language) = cfg.language {
        language
    } else if let Some(language) = guess_language(&source, &path).0 {
        language
    } else {
        return Ok(());
    };

    let pr = cfg.preproc.clone();
    if cfg.dump {
        let cfg = DumpCfg {
            line_start: cfg.line_start,
            line_end: cfg.line_end,
        };
        action::<Dump>(&language, source, &path, pr, cfg)
    } else if cfg.metrics {
        if let Some(output_format) = &cfg.output_format {
            if let Some(space) = get_function_spaces(&language, source, &path, pr) {
                save_funcspace("".to_string(), &space);
                output_format.dump_formats(&space, &path, &cfg.output, cfg.pretty)
            } else {
                Ok(())
            }
        } else {
            let cfg = MetricsCfg { path };
            let path = cfg.path.clone();
            action::<Metrics>(&language, source, &path, pr, cfg)
        }
    } else if cfg.ops {
        if let Some(output_format) = &cfg.output_format {
            let ops = get_ops(&language, source, &path, pr).unwrap();
            output_format.dump_formats(&ops, &path, &cfg.output, cfg.pretty)
        } else {
            let cfg = OpsCfg { path };
            let path = cfg.path.clone();
            action::<OpsCode>(&language, source, &path, pr, cfg)
        }
    } else if cfg.comments {
        let cfg = CommentRmCfg {
            in_place: cfg.in_place,
            path,
        };
        let path = cfg.path.clone();
        if language == LANG::Cpp {
            action::<CommentRm>(&LANG::Ccomment, source, &path, pr, cfg)
        } else {
            action::<CommentRm>(&language, source, &path, pr, cfg)
        }
    } else if cfg.function {
        let cfg = FunctionCfg { path: path.clone() };
        action::<Function>(&language, source, &path, pr, cfg)
    } else if !cfg.find_filter.is_empty() {
        let cfg = FindCfg {
            path: path.clone(),
            filters: cfg.find_filter.clone(),
            line_start: cfg.line_start,
            line_end: cfg.line_end,
        };
        action::<Find>(&language, source, &path, pr, cfg)
    } else if cfg.count_lock.is_some() {
        let cfg = CountCfg {
            filters: cfg.count_filter.clone(),
            stats: cfg.count_lock.as_ref().unwrap().clone(),
        };
        action::<Count>(&language, source, &path, pr, cfg)
    } else if cfg.preproc_lock.is_some() {
        if let Some(language) = guess_language(&source, &path).0 {
            if language == LANG::Cpp {
                let mut results = cfg.preproc_lock.as_ref().unwrap().lock().unwrap();
                preprocess(
                    &PreprocParser::new(source, &path, None),
                    &path,
                    &mut results,
                );
            }
        }
        Ok(())
    } else {
        Ok(())
    }
}

// fixme: copy and edit from rust-code-analysis
fn process_dir_path(all_files: &mut HashMap<String, Vec<PathBuf>>, path: &Path, cfg: &Config) {
    if cfg.preproc_lock.is_some() {
        let file_name = path.file_name().unwrap().to_str().unwrap().to_string();
        match all_files.entry(file_name) {
            hash_map::Entry::Occupied(l) => {
                l.into_mut().push(path.to_path_buf());
            }
            hash_map::Entry::Vacant(p) => {
                p.insert(vec![path.to_path_buf()]);
            }
        };
    }
}

// fixme: copy and edit from rust-code-analysis
#[derive(Parser, Debug)]
#[clap(
    name = "rust-code-analysis-cli",
    version,
    author,
    about = "Analyze source code."
)]
struct Opts {
    /// Input files to analyze.
    #[clap(long, short, value_parser)]
    paths: Vec<PathBuf>,
    /// Output AST to stdout.
    #[clap(long, short)]
    dump: bool,
    /// Remove comments in the specified files.
    #[clap(long, short)]
    comments: bool,
    /// Find nodes of the given type.
    #[clap(long, short, default_value = "Vec::new()", number_of_values = 1)]
    find: Vec<String>,
    /// Get functions and their spans.
    #[clap(long, short = 'F')]
    function: bool,
    /// Count nodes of the given type: comma separated list.
    #[clap(long, short = 'C', default_value = "Vec::new()", number_of_values = 1)]
    count: Vec<String>,
    /// Compute different metrics.
    #[clap(long, short)]
    metrics: bool,
    /// Retrieve all operands and operators in a code.
    #[clap(long, conflicts_with = "metrics")]
    ops: bool,
    /// Do action in place.
    #[clap(long, short)]
    in_place: bool,
    /// Glob to include files.
    #[clap(long, short = 'I')]
    include: Vec<String>,
    /// Glob to exclude files.
    #[clap(long, short = 'X')]
    exclude: Vec<String>,
    /// Number of jobs.
    #[clap(long, short = 'j')]
    num_jobs: Option<usize>,
    /// Language type.
    #[clap(long, short)]
    language_type: Option<String>,
    /// Output metrics as different formats.
    #[clap(long, short = 'O', possible_values = Format::all())]
    output_format: Option<Format>,
    /// Dump a pretty json file.
    #[clap(long = "pr")]
    pretty: bool,
    /// Output file/directory.
    #[clap(long, short, value_parser)]
    output: Option<PathBuf>,
    /// Get preprocessor declaration for C/C++.
    #[clap(long, value_parser, number_of_values = 1)]
    preproc: Vec<PathBuf>,
    /// Line start.
    #[clap(long = "ls")]
    line_start: Option<usize>,
    /// Line end.
    #[clap(long = "le")]
    line_end: Option<usize>,
    /// Print the warnings.
    #[clap(long, short)]
    warning: bool,
}

/// rust-code-analysis 检查项，属于代码度量
#[derive(Debug)]
pub struct RustCodeAnalysis {
    pub project_cfg: config::ProjectInfoConfig,
    pub config: config::Config,
    pub dir_list: HashMap<PathBuf, DirEntry>,
    pub func_info: Vec<FuncInfo>,
    pub check_result: Option<result::CheckResultDetail>,
    pub parse_result: Option<result::ParseResultDetail>,
    pub count_result: Option<result::CountResultDetail>,
}

impl super::CheckToolOption for RustCodeAnalysis {
    // fixme: copy and edit from rust-code-analysis/rust-code-analysis-cli
    fn check(&mut self) -> Result<()> {
        log::info!("RustCodeAnalysis check: {:?}", self.project_cfg);
        // Get subdirectories under the project.
        let exclude_dir = self.config.exclude_dir.clone().unwrap_or_default();
        self.dir_list = util::get_all_dir(
            self.project_cfg.dir.clone(),
            r"src",
            exclude_dir,
            util::dir_and_name,
        )?;
        log::info!(
            "RustCodeAnalysis check path: {:?} paths:{:?}",
            self.project_cfg.dir.clone(),
            self.dir_list
        );
        // Used to store the scan results of each source file.
        let mut output = self.project_cfg.result.clone();
        output.pop();
        output.push("details");
        std::fs::create_dir_all(&output)
            .with_context(|| format!("Create result dir path failed! {:?}", output))?;

        let opts = Opts {
            paths: self.dir_list.clone().into_keys().collect(),
            dump: false,
            comments: false,
            find: vec!["Vec::new()".to_owned()],
            function: false,
            count: vec!["Vec::new()".to_owned()],
            metrics: true,
            ops: false,
            in_place: false,
            include: vec![],
            exclude: vec![],
            num_jobs: None,
            language_type: None,
            output_format: Some(Format::Toml),
            pretty: false,
            output: Some(output),
            preproc: vec![],
            line_start: None,
            line_end: None,
            warning: false,
        };
        let count_lock = if !opts.count.is_empty() {
            Some(Arc::new(Mutex::new(Count::default())))
        } else {
            None
        };

        let (preproc_lock, preproc) = match opts.preproc.len().cmp(&1) {
            Ordering::Equal => {
                let data = read_file(&opts.preproc[0]).unwrap();
                log::info!("Load preproc data");
                let x = (
                    None,
                    Some(Arc::new(
                        serde_json::from_slice::<PreprocResults>(&data).unwrap(),
                    )),
                );
                log::info!("Load preproc data: finished");
                x
            }
            Ordering::Greater => (Some(Arc::new(Mutex::new(PreprocResults::default()))), None),
            Ordering::Less => (None, None),
        };

        let output_is_dir = opts.output.as_ref().map(|p| p.is_dir()).unwrap_or(false);
        if (opts.metrics || opts.ops) && opts.output.is_some() && !output_is_dir {
            log::info!("Error: The output parameter must be a directory");
            process::exit(1);
        }

        let typ = opts.language_type.unwrap_or_default();
        let language = if preproc_lock.is_some() {
            Some(LANG::Preproc)
        } else if typ.is_empty() {
            None
        } else if typ == "ccomment" {
            Some(LANG::Ccomment)
        } else if typ == "preproc" {
            Some(LANG::Preproc)
        } else {
            get_from_ext(&typ)
        };

        let num_jobs = opts
            .num_jobs
            .map(|num_jobs| std::cmp::max(2, num_jobs) - 1)
            .unwrap_or_else(|| {
                std::cmp::max(
                    2,
                    available_parallelism()
                        .expect("Unrecoverable: Failed to get thread count")
                        .get(),
                ) - 1
            });

        let include = mk_globset(opts.include);
        let exclude = mk_globset(opts.exclude);

        let cfg = Config {
            dump: opts.dump,
            in_place: opts.in_place,
            comments: opts.comments,
            find_filter: opts.find,
            count_filter: opts.count,
            language,
            function: opts.function,
            metrics: opts.metrics,
            ops: opts.ops,
            output_format: opts.output_format,
            pretty: opts.pretty,
            output: opts.output.clone(),
            line_start: opts.line_start,
            line_end: opts.line_end,
            preproc_lock,
            preproc,
            count_lock,
        };

        let files_data = FilesData {
            include,
            exclude,
            paths: opts.paths,
        };

        match ConcurrentRunner::new(num_jobs, act_on_file)
            .set_proc_dir_paths(process_dir_path)
            .run(cfg, files_data)
        {
            Ok(_) => {}
            Err(e) => {
                log::info!("{:?}", e);
                process::exit(1);
            }
        };
        Ok(())
    }

    /// Parse the rust-code-analysis measure result into the format suitable
    /// for users and calculation.
    fn parse(&mut self) -> Result<()> {
        log::info!(
            "RustCodeAnalysis parse: FUNC_SPACE_RESULT len: {:?}",
            FUNC_SPACE_RESULT.lock().unwrap().len()
        );
        if let Some(config::QualityEvaluationConfig {
            measeure_cfg: Some(m),
            ..
        }) = &self.config.quality_evaluation_cfg
        {
            let mut large_cyclomatic_complexity: u64 = 0;
            let mut large_cognitive_complexity: u64 = 0;
            let mut large_num_rows_file: u64 = 0;
            let mut large_num_rows_function: u64 = 0;
            let mut func_space = FUNC_SPACE_RESULT.lock().unwrap();
            for v in func_space.iter_mut() {
                if v.kind == FuncKind::Function {
                    // for function
                    let func_len = v.end_line - v.start_line;
                    if func_len > m.large_num_rows_function.unwrap() as usize {
                        large_num_rows_function += 1;
                        v.if_large_function = true;
                    }
                    if v.cognitive > m.large_cognitive_complexity.unwrap() as usize {
                        large_cognitive_complexity += 1;
                        v.if_large_cognitive = true;
                    }
                    if v.cyclomatic > m.large_cyclomatic_complexity.unwrap() as usize {
                        large_cyclomatic_complexity += 1;
                        v.if_large_cyclomatic = true;
                    }
                } else {
                    // for the kind of file
                    let file_len = v.end_line - v.start_line;
                    if file_len > m.large_num_rows_file.unwrap() as usize {
                        large_num_rows_file += 1;
                        v.if_large_file = true;
                    }
                }
                self.func_info.push(v.clone());
            }
            self.parse_result = Some(result::ParseResultDetail::CodeAnalysis {
                large_cyclomatic_complexity: Some(large_cyclomatic_complexity),
                large_cognitive_complexity: Some(large_cognitive_complexity),
                large_num_rows_file: Some(large_num_rows_file),
                large_num_rows_function: Some(large_num_rows_function),
            });
        }

        self.write_result_file()?;
        Ok(())
    }

    /// Calculate the measure score
    fn count(&mut self) -> Result<()> {
        log::info!("RustCodeAnalysis count: {:?}", self.parse_result);
        if let Some(config::QualityEvaluationConfig {
            measeure_cfg:
                Some(config::MeasureEvaluationConfig {
                    large_cyclomatic_complexity_score: Some(large_cyclomatic_complexity_score_cfg),
                    large_cognitive_complexity_score: Some(large_cognitive_complexity_score_cfg),
                    large_num_rows_function_score: Some(large_num_rows_function_score_cfg),
                    large_num_rows_file_score: Some(large_num_rows_file_score_cfg),
                    measure_score: Some(measure_score_cfg),
                    measure_weight: Some(measure_weight_cfg),
                    ..
                }),
            ..
        }) = self.config.quality_evaluation_cfg
        {
            if let Some(result::ParseResultDetail::CodeAnalysis {
                large_cyclomatic_complexity: Some(large_cyclomatic_complexity),
                large_cognitive_complexity: Some(large_cognitive_complexity),
                large_num_rows_file: Some(large_num_rows_file),
                large_num_rows_function: Some(large_num_rows_function),
            }) = &self.parse_result
            {
                let score = measure_score_cfg
                    .saturating_sub(
                        large_cyclomatic_complexity_score_cfg * large_cyclomatic_complexity,
                    )
                    .saturating_sub(
                        large_cognitive_complexity_score_cfg * large_cognitive_complexity,
                    )
                    .saturating_sub(large_num_rows_file * large_num_rows_file_score_cfg)
                    .saturating_sub(large_num_rows_function * large_num_rows_function_score_cfg)
                    * 100
                    / measure_score_cfg;
                self.count_result = Some(result::CountResultDetail::CodeAnalysis {
                    score: Some(score),
                    normalized_score: Some(score * measure_weight_cfg / 100),
                });
            }
        }
        Ok(())
    }

    /// Process the results presented to the user
    fn result(&mut self, result: &mut result::Result) -> Result<()> {
        log::info!("RustCodeAnalysis result: {:?}", self.count_result);
        if let Some(result::CountResultDetail::CodeAnalysis {
            score: Some(score),
            normalized_score: Some(n_score),
        }) = self.count_result
        {
            result.code_measure = Some(result::CodeMeasure {
                score: Some(score),
                normalized_score: Some(n_score),
            })
        }

        Ok(())
    }
}

impl RustCodeAnalysis {
    fn write_result_file(&self) -> Result<()> {
        let _ = File::create(&self.project_cfg.result).unwrap();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.project_cfg.result)
            .unwrap();
        for v in self.func_info.iter() {
            file.write_all(toml::to_string(v).unwrap().as_bytes())?;
            file.write_all("\n".as_bytes())?;
        }
        Ok(())
    }
}
