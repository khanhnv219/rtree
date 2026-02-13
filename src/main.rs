use clap::{Parser, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(name = "rtree", version, about = "Fast disk usage analyzer for files and directories")]
struct Cli {
    /// Target path to analyze
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Sort by size (default) or name
    #[arg(long, value_enum, default_value_t = SortBy::Size)]
    sort: SortBy,

    /// Limit output to top N items
    #[arg(short = 'n', long = "limit")]
    limit: Option<usize>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
enum SortBy {
    Size,
    Name,
}

#[derive(Debug)]
struct ItemStat {
    path: PathBuf,
    size: u64,
    is_dir: bool,
}

fn main() {
    let cli = Cli::parse();

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] files scanned: {pos}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_message("Scanning...");

    let scan_result = collect_stats(&cli.path, Arc::new(pb.clone()));
    pb.finish_and_clear();

    let mut items = match scan_result {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "Failed to scan '{}': {}",
                cli.path.to_string_lossy(),
                format_io_error(&e)
            );
            std::process::exit(1);
        }
    };

    sort_items(&mut items, cli.sort);

    if let Some(limit) = cli.limit {
        if items.len() > limit {
            items.truncate(limit);
        }
    }

    print_table(&items);
}

fn collect_stats(target: &Path, pb: Arc<ProgressBar>) -> io::Result<Vec<ItemStat>> {
    let meta = fs::metadata(target)?;

    if meta.is_file() {
        pb.inc(1);
        return Ok(vec![ItemStat {
            path: target.to_path_buf(),
            size: meta.len(),
            is_dir: false,
        }]);
    }

    let entries: Vec<PathBuf> = fs::read_dir(target)?
        .filter_map(|entry_res| match entry_res {
            Ok(entry) => Some(entry.path()),
            Err(err) => {
                if !is_permission_denied(&err) {
                    eprintln!("Warning: could not read an entry in '{}': {}", target.to_string_lossy(), format_io_error(&err));
                }
                None
            }
        })
        .collect();

    let stats: Vec<ItemStat> = entries
        .into_par_iter()
        .map(|path| {
            let result = stat_path(&path, &pb);
            match result {
                Ok(stat) => Some(stat),
                Err(err) => {
                    if !is_permission_denied(&err) {
                        eprintln!(
                            "Warning: failed to scan '{}': {}",
                            path.to_string_lossy(),
                            format_io_error(&err)
                        );
                    }
                    None
                }
            }
        })
        .flatten()
        .collect();

    Ok(stats)
}

fn stat_path(path: &Path, pb: &ProgressBar) -> io::Result<ItemStat> {
    let meta = fs::metadata(path)?;
    if meta.is_file() {
        pb.inc(1);
        return Ok(ItemStat {
            path: path.to_path_buf(),
            size: meta.len(),
            is_dir: false,
        });
    }

    let size = walk_size(path, pb);
    Ok(ItemStat {
        path: path.to_path_buf(),
        size,
        is_dir: true,
    })
}

fn walk_size(path: &Path, pb: &ProgressBar) -> u64 {
    let mut total = 0u64;

    for entry in WalkDir::new(path).follow_links(false).into_iter() {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                if let Some(ioe) = err.io_error() {
                    if !is_permission_denied(ioe) {
                        eprintln!(
                            "Warning: traversal issue under '{}': {}",
                            path.to_string_lossy(),
                            format_io_error(ioe)
                        );
                    }
                }
                continue;
            }
        };

        if entry.file_type().is_file() {
            pb.inc(1);
            match entry.metadata() {
                Ok(md) => total = total.saturating_add(md.len()),
                Err(err) => {
                    if let Some(ioe) = err.io_error() {
                        if !is_permission_denied(ioe) {
                            eprintln!(
                                "Warning: metadata read failed for '{}': {}",
                                entry.path().to_string_lossy(),
                                format_io_error(ioe)
                            );
                        }
                    } else {
                        eprintln!(
                            "Warning: metadata read failed for '{}': {}",
                            entry.path().to_string_lossy(),
                            err
                        );
                    }
                }
            }
        }
    }

    total
}

fn sort_items(items: &mut [ItemStat], sort: SortBy) {
    match sort {
        SortBy::Size => {
            items.sort_by(|a, b| {
                b.size
                    .cmp(&a.size)
                    .then_with(|| a.path.to_string_lossy().cmp(&b.path.to_string_lossy()))
            });
        }
        SortBy::Name => {
            items.sort_by(|a, b| {
                a.path
                    .to_string_lossy()
                    .cmp(&b.path.to_string_lossy())
                    .then_with(|| b.size.cmp(&a.size))
            });
        }
    }
}

fn print_table(items: &[ItemStat]) {
    if items.is_empty() {
        println!("No items found.");
        return;
    }

    let size_strings: Vec<String> = items.iter().map(|i| human_size(i.size)).collect();
    let size_width = size_strings
        .iter()
        .map(|s| s.len())
        .max()
        .unwrap_or(4)
        .max(4);

    println!(
        "{:<size_width$}  {:<4}  {}",
        "Size",
        "Type",
        "Path",
        size_width = size_width
    );
    println!("{}", "-".repeat(size_width + 2 + 4 + 2 + 40));

    for (item, size_str) in items.iter().zip(size_strings.iter()) {
        let kind = if item.is_dir { "DIR" } else { "FILE" };
        println!(
            "{:<size_width$}  {:<4}  {}",
            size_str,
            kind,
            item.path.to_string_lossy(),
            size_width = size_width
        );
    }
}

fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];

    if bytes < 1024 {
        return format!("{bytes} B");
    }

    let mut value = bytes as f64;
    let mut idx = 0usize;
    while value >= 1024.0 && idx < UNITS.len() - 1 {
        value /= 1024.0;
        idx += 1;
    }

    format!("{value:.2} {}", UNITS[idx])
}

fn is_permission_denied(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::PermissionDenied
}

fn format_io_error(err: &io::Error) -> String {
    if let Some(code) = err.raw_os_error() {
        format!("{} (os error {})", err, code)
    } else {
        err.to_string()
    }
}
