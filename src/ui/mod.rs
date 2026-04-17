pub mod list;
pub mod tui;

use anyhow::Result;
use std::io::{self, Write};

use crate::core::model::Package;

pub fn confirm_removal(packages: &[Package]) -> Result<bool> {
    let total_size: u64 = packages.iter().map(|p| p.size).sum();
    let total_mb = total_size as f64 / 1_048_576.0;

    println!(
        "\nAbout to remove {} packages, freeing {:.1} MiB",
        packages.len(),
        total_mb
    );
    print!("Continue? [y/N]: ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    Ok(input.trim().to_lowercase() == "y" || input.trim().to_lowercase() == "yes")
}
