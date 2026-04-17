use anyhow::Result;
use clap::{Parser, Subcommand};
use std::process;

mod config;
mod core;
mod exec;
mod ui;

use crate::core::alpm::AlpmContext;
use crate::ui::tui::TuiApp;

#[derive(Parser)]
#[command(name = "dude")]
#[command(about = "Find and remove orphan packages on Arch Linux")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    #[arg(long, global = true)]
    keep: Option<String>,
    #[arg(long, global = true)]
    nosave: bool,
    #[arg(long, global = true)]
    hook: bool,
}

#[derive(Subcommand)]
enum Commands {
    List,
    Tui,
    Prune {
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        dry: bool,
    },
    Auto,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let cfg = config::Config::load()?;
    let alpm = AlpmContext::new()?;
    let mut orphans = alpm.get_orphans()?;
    if let Some(pat) = &cli.keep {
        orphans = cfg.filter_keep_pattern(&orphans, pat)?;
    }
    orphans = cfg.filter_whitelist(&orphans);
    if orphans.is_empty() {
        if !cli.hook {
            println!("No orphan packages found.");
        }
        return Ok(());
    }

    match cli.command {
        Some(Commands::List) => ui::list::show_orphans(&orphans),
        Some(Commands::Tui) => {
            let mut app = TuiApp::new(orphans);
            app.run()?;
            let sel = app.selected_packages();
            if !sel.is_empty() {
                exec::tx::remove_packages(&sel, cli.nosave)?;
            }
        }
        Some(Commands::Auto) => {
            if cfg.should_auto_prune(&orphans) {
                println!("Auto-pruning {} orphan packages…", orphans.len());
                exec::tx::remove_packages(&orphans, cli.nosave)?;
                cfg.update_last_run()?;
            } else if !cli.hook {
                println!("Auto-prune conditions not met.");
            }
        }
        Some(Commands::Prune { yes, dry }) => {
            if dry || (!yes && !atty::is(atty::Stream::Stdout)) {
                ui::list::show_orphans(&orphans);
                println!("\nDry run – no packages removed.");
            } else if yes {
                exec::tx::remove_packages(&orphans, cli.nosave)?;
            } else {
                ui::list::show_orphans(&orphans);
                if ui::confirm_removal(&orphans)? {
                    exec::tx::remove_packages(&orphans, cli.nosave)?;
                }
            }
        }
        None => {
            let mut app = TuiApp::new(orphans);
            app.run()?;
            let sel = app.selected_packages();
            if !sel.is_empty() {
                exec::tx::remove_packages(&sel, cli.nosave)?;
            }
        }
    }
    Ok(())
}
