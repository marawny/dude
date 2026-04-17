use crate::core::model::Package;

pub fn show_orphans(packages: &[Package]) {
    if packages.is_empty() {
        println!("No orphan packages found.");
        return;
    }

    let total_size: u64 = packages.iter().map(|p| p.size).sum();
    let total_mb = total_size as f64 / 1_048_576.0;

    println!(
        "Found {} orphan packages ({:.1} MiB total):\n",
        packages.len(),
        total_mb
    );

    for pkg in packages {
        println!("  {}", pkg);
    }

    println!();
}
