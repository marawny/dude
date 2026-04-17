use anyhow::{anyhow, Result};
use nix::unistd::geteuid;
use std::process::Command;

use crate::core::model::Package;

pub fn remove_packages(packages: &[Package], nosave: bool) -> Result<()> {
    if packages.is_empty() {
        return Ok(());
    }

    if geteuid().is_root() {
        return remove_with_pacman(packages, nosave);
    }

    let mut cmd = Command::new("sudo");
    cmd.arg("pacman");
    cmd.arg(if nosave { "-Rns" } else { "-Rs" });
    cmd.args(packages.iter().map(|p| p.name.as_str()));

    println!(
        "Executing: sudo pacman {} {}",
        if nosave { "-Rns" } else { "-Rs" },
        packages
            .iter()
            .map(|p| p.name.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    );

    let status = cmd.status()?;

    if status.success() {
        println!("\n✓ Successfully removed {} packages", packages.len());
        #[cfg(feature = "notifications")]
        if let Err(e) = send_notification(packages.len()) {
            eprintln!("Warning: Failed to send notification: {}", e);
        }
        Ok(())
    } else {
        Err(anyhow!("Failed to remove packages"))
    }
}

fn remove_with_pacman(packages: &[Package], nosave: bool) -> Result<()> {
    let mut cmd = Command::new("pacman");
    cmd.arg(if nosave { "-Rns" } else { "-Rs" });
    cmd.args(packages.iter().map(|p| p.name.as_str()));

    let status = cmd.status()?;
    if status.success() {
        println!("\n✓ Successfully removed {} packages", packages.len());
        Ok(())
    } else {
        Err(anyhow!("Failed to remove packages"))
    }
}

#[cfg(feature = "notifications")]
fn send_notification(count: usize) -> Result<()> {
    notify_rust::Notification::new()
        .summary("dude")
        .body(&format!("Successfully removed {} orphan packages", count))
        .icon("package-x-generic")
        .timeout(5000)
        .show()?;
    Ok(())
}
