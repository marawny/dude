use anyhow::{anyhow, Context, Result};
use std::process::{Command, Output};

use crate::core::model::Package;

pub struct AlpmContext;

impl AlpmContext {
    pub fn new() -> Result<Self> {
        Ok(Self)
    }

    /// True orphans: packages installed as dependencies that are neither required
    /// nor optionally required by any installed package.
    pub fn get_orphans(&self) -> Result<Vec<Package>> {
        let mut orphans = query_orphan_names()?
            .into_iter()
            .map(|name| load_package(&name))
            .collect::<Result<Vec<_>>>()?;

        orphans.sort_by(|a, b| b.size.cmp(&a.size));
        Ok(orphans)
    }
}

fn query_orphan_names() -> Result<Vec<String>> {
    let output = run_pacman(&["-Qdttq"])?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    if output.status.success() || (output.status.code() == Some(1) && stdout.trim().is_empty()) {
        return Ok(parse_orphan_names(&stdout));
    }

    Err(command_error("pacman -Qdttq", &output))
}

fn load_package(name: &str) -> Result<Package> {
    let info = run_pacman(&["-Qi", name])?;
    if !info.status.success() {
        return Err(command_error(&format!("pacman -Qi {name}"), &info));
    }

    let repo = match run_pacman(&["-Si", name]) {
        Ok(sync_info) if sync_info.status.success() => {
            parse_repository(&String::from_utf8_lossy(&sync_info.stdout))
                .unwrap_or_else(|| "local".to_string())
        }
        _ => "local".to_string(),
    };

    Package::from_pacman_query(&String::from_utf8_lossy(&info.stdout), repo)
        .with_context(|| format!("failed to parse package metadata for {name}"))
}

fn run_pacman(args: &[&str]) -> Result<Output> {
    Command::new("pacman")
        .env("LC_ALL", "C")
        .env("LANG", "C")
        .args(args)
        .output()
        .with_context(|| format!("failed to run pacman {}", args.join(" ")))
}

fn parse_orphan_names(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn parse_repository(sync_info: &str) -> Option<String> {
    sync_info.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key.trim() == "Repository").then(|| value.trim().to_string())
    })
}

fn command_error(command: &str, output: &Output) -> anyhow::Error {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if stderr.is_empty() { stdout } else { stderr };
    anyhow!("{command} failed: {detail}")
}

#[cfg(test)]
mod tests {
    use super::{parse_orphan_names, parse_repository};

    #[test]
    fn parses_orphan_names_from_query_output() {
        let names = parse_orphan_names("go\n\npython-installer\n");
        assert_eq!(names, vec!["go", "python-installer"]);
    }

    #[test]
    fn parses_repository_from_sync_info() {
        let repo = parse_repository(
            "Repository      : extra\nName            : ripgrep\nVersion         : 14.1.1-1\n",
        );
        assert_eq!(repo.as_deref(), Some("extra"));
    }
}
