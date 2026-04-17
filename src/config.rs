use anyhow::Result;
use chrono::{TimeZone, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::core::model::Package;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub whitelist: Vec<String>,
    #[serde(default)]
    pub auto_prune: Option<AutoPruneConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AutoPruneConfig {
    pub threshold_mb: u64,
    pub days_since_last_run: u32,
}

impl Config {
    pub fn load() -> Result<Self> {
        let mut config = Self::default();
        if let Ok(content) = fs::read_to_string("/etc/dude.conf") {
            if let Ok(sys) = toml::from_str::<Config>(&content) {
                config.merge(sys);
            }
        }
        if let Some(dir) = dirs::config_dir() {
            let path = dir.join("dude").join("config");
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(user) = toml::from_str::<Config>(&content) {
                    config.merge(user);
                }
            }
        }
        Ok(config)
    }

    fn merge(&mut self, other: Config) {
        self.whitelist.extend(other.whitelist);
        if other.auto_prune.is_some() {
            self.auto_prune = other.auto_prune;
        }
    }

    pub fn filter_whitelist(&self, pkgs: &[Package]) -> Vec<Package> {
        pkgs.iter()
            .filter(|p| !self.whitelist.contains(&p.name))
            .cloned()
            .collect()
    }

    pub fn filter_keep_pattern(&self, pkgs: &[Package], pat: &str) -> Result<Vec<Package>> {
        let re = Regex::new(pat)?;
        Ok(pkgs
            .iter()
            .filter(|p| !re.is_match(&p.name))
            .cloned()
            .collect())
    }

    pub fn should_auto_prune(&self, pkgs: &[Package]) -> bool {
        let Some(cfg) = &self.auto_prune else {
            return false;
        };
        let total_mb: f64 = pkgs.iter().map(|p| p.size).sum::<u64>() as f64 / 1_048_576.0;
        if total_mb < cfg.threshold_mb as f64 {
            return false;
        }
        let state = self.state_file();
        if let Ok(raw) = fs::read_to_string(&state) {
            if let Ok(ts) = raw.trim().parse::<i64>() {
                if let Some(dt) = Utc.timestamp_opt(ts, 0).single() {
                    if (Utc::now() - dt).num_days() < cfg.days_since_last_run as i64 {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub fn update_last_run(&self) -> Result<()> {
        let state = self.state_file();
        if let Some(parent) = state.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(state, Utc::now().timestamp().to_string())?;
        Ok(())
    }

    fn state_file(&self) -> PathBuf {
        dirs::state_dir()
            .unwrap_or_else(|| dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp")))
            .join("dude")
            .join("last_run")
    }
}
