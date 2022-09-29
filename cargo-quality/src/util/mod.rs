use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use walkdir::{DirEntry, WalkDir};

/// Get the specified file under the path.
pub fn dir_and_name(d: &DirEntry, name: &str, exclude: Vec<String>) -> bool {
    if let Some(_) = exclude
        .iter()
        .find(|&name| d.path().as_os_str().to_str().unwrap().contains(name))
    {
        return false;
    }
    return d.file_type().is_dir() && d.path().as_os_str().to_str().unwrap().ends_with(name);
}

/// Get all subdirectories under the path.
pub fn get_all_dir(
    p: PathBuf,
    name: &str,
    exclude: Vec<String>,
    op: fn(e: &DirEntry, name: &str, exclude: Vec<String>) -> bool,
) -> Result<HashMap<PathBuf, DirEntry>> {
    let mut dir_result = HashMap::new();
    for e in WalkDir::new(p)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|dir| op(dir, name, exclude.clone()))
    {
        dir_result.insert(e.path().to_path_buf(), e);
    }
    Ok(dir_result)
}

/// Standard output type.
#[derive(Debug)]
pub enum StdOutput {
    Out,
    Err,
}

/// Run the cargo subcommand.
pub fn cargo(
    sub_command: &str,
    manifest_file: &PathBuf,
    result_file: &PathBuf,
    std_info: StdOutput,
) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.arg(sub_command)
        .arg("--manifest-path")
        .arg(manifest_file);
    let output = cmd
        .output()
        .with_context(|| format!("Failed to run command: {:?}", cmd))?;
    // `cargo clippy` will fail when has errors.
    if !output.status.success() {
        println!("Run the cargo subcommand : {:?} failed.", sub_command);
    }
    let mut f = File::create(result_file)?;
    match std_info {
        StdOutput::Out => f.write_all(&output.stdout)?,
        StdOutput::Err => f.write_all(&output.stderr)?,
    };
    Ok(())
}
