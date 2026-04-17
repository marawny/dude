use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use std::fmt;

#[derive(Debug, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub size: u64,
    pub repo: String,
    pub install_date: DateTime<Utc>,
}

impl Package {
    pub fn from_pacman_query(info: &str, repo: String) -> Result<Self> {
        let name = field(info, "Name")?.to_string();
        let version = field(info, "Version")?.to_string();
        let size = parse_installed_size(field(info, "Installed Size")?)?;
        let install_date = parse_install_date(field(info, "Install Date")?)?;

        Ok(Self {
            name,
            version,
            size,
            repo,
            install_date,
        })
    }

    pub fn size_human(&self) -> String {
        let b = self.size as f64;
        if b >= 1_073_741_824.0 {
            format!("{:.1} GiB", b / 1_073_741_824.0)
        } else if b >= 1_048_576.0 {
            format!("{:.1} MiB", b / 1_048_576.0)
        } else if b >= 1024.0 {
            format!("{:.1} KiB", b / 1024.0)
        } else {
            format!("{} B", b as u64)
        }
    }

    pub fn repo_color(&self) -> &'static str {
        match self.repo.as_str() {
            "core" => "\x1b[31m",
            "extra" => "\x1b[32m",
            "community" | "community-testing" => "\x1b[34m",
            "multilib" => "\x1b[35m",
            _ => "\x1b[33m",
        }
    }
}

fn field<'a>(info: &'a str, name: &str) -> Result<&'a str> {
    info.lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            (key.trim() == name).then_some(value.trim())
        })
        .ok_or_else(|| anyhow!("missing `{name}` field"))
}

fn parse_installed_size(raw: &str) -> Result<u64> {
    let mut parts = raw.split_whitespace();
    let value: f64 = parts
        .next()
        .context("missing installed size value")?
        .parse()
        .context("invalid installed size value")?;
    let unit = parts.next().unwrap_or("B");

    let bytes = match unit {
        "B" => value,
        "KiB" => value * 1024.0,
        "MiB" => value * 1_048_576.0,
        "GiB" => value * 1_073_741_824.0,
        _ => return Err(anyhow!("unsupported size unit `{unit}`")),
    };

    Ok(bytes.round() as u64)
}

fn parse_install_date(raw: &str) -> Result<DateTime<Utc>> {
    let naive = NaiveDateTime::parse_from_str(raw, "%a %b %e %H:%M:%S %Y")
        .with_context(|| format!("invalid install date `{raw}`"))?;
    Ok(Utc.from_utc_datetime(&naive))
}

impl fmt::Display for Package {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}\x1b[0m {} ({}) - {} - {}",
            self.repo_color(),
            self.name,
            self.version,
            self.size_human(),
            self.repo,
            self.install_date.format("%Y-%m-%d")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_install_date, parse_installed_size, Package};

    #[test]
    fn parses_package_from_pacman_query_output() {
        let package = Package::from_pacman_query(
            "Name            : bash\nVersion         : 5.3.9-1\nInstalled Size  : 9.56 MiB\nInstall Date    : Fri Apr 17 20:21:03 2026\n",
            "core".to_string(),
        )
        .expect("package metadata should parse");

        assert_eq!(package.name, "bash");
        assert_eq!(package.version, "5.3.9-1");
        assert_eq!(package.repo, "core");
        assert_eq!(package.size, 10_024_387);
        assert_eq!(
            package.install_date.format("%Y-%m-%d").to_string(),
            "2026-04-17"
        );
    }

    #[test]
    fn parses_installed_size_units() {
        assert_eq!(parse_installed_size("512 B").unwrap(), 512);
        assert_eq!(parse_installed_size("1.5 KiB").unwrap(), 1536);
        assert_eq!(parse_installed_size("2.0 MiB").unwrap(), 2_097_152);
        assert_eq!(parse_installed_size("3.0 GiB").unwrap(), 3_221_225_472);
    }

    #[test]
    fn parses_install_dates_in_pacman_format() {
        let parsed = parse_install_date("Fri Apr 17 20:21:03 2026").unwrap();
        assert_eq!(
            parsed.format("%Y-%m-%d %H:%M:%S").to_string(),
            "2026-04-17 20:21:03"
        );
    }
}
