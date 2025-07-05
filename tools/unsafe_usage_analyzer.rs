#!/usr/bin/env -S cargo +nightly -Zscript
---
[package]
name = "unsafe_usage_analyzer"
version = "0.1.0"
edition = "2024"

[dependencies]
syn = { version = "2.0.104", features = ["full", "visit"] }
walkdir = "2.5.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = { version = "0.4", features = ["serde"] }
---

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use syn::{ExprUnsafe, ItemFn, visit::Visit};
use walkdir::WalkDir;

fn colorize_percentage(unsafe_count: usize, total_count: usize) -> String {
    if total_count == 0 {
        format!("\x1b[90m{}/{}\x1b[0m", unsafe_count, total_count)
    } else if unsafe_count == 0 {
        format!("\x1b[32m{}/{}\x1b[0m", unsafe_count, total_count)
    } else {
        let percentage = (unsafe_count as f64 / total_count as f64) * 100.0;
        if percentage < 50.0 {
            format!("\x1b[33m{}/{}\x1b[0m", unsafe_count, total_count)
        } else {
            format!("\x1b[31m{}/{}\x1b[0m", unsafe_count, total_count)
        }
    }
}

#[derive(Default, Serialize, Deserialize, Clone)]
struct UnsafeStats {
    unsafe_fns: usize,
    total_fns: usize,
    unsafe_lines: usize,
}

#[derive(Default, Serialize, Deserialize, Clone)]
struct FileStats {
    filename: String,
    stats: UnsafeStats,
}

#[derive(Serialize, Deserialize, Clone)]
struct Report {
    files: Vec<FileStats>,
    total: UnsafeStats,
    timestamp: String,
}

struct UnsafeAnalyzer<'a> {
    stats: &'a mut UnsafeStats,
}

impl<'a, 'ast> Visit<'ast> for UnsafeAnalyzer<'a> {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        self.stats.total_fns += 1;
        if i.sig.unsafety.is_some() {
            self.stats.unsafe_fns += 1;
        }
        syn::visit::visit_item_fn(self, i);
    }

    fn visit_expr_unsafe(&mut self, i: &'ast ExprUnsafe) {
        // Count lines in unsafe block - use a simple heuristic since span info isn't available
        // Count the number of statements and expressions in the unsafe block
        self.stats.unsafe_lines += 1 + i.block.stmts.len();

        syn::visit::visit_expr_unsafe(self, i);
    }
}

fn analyze_file(path: &Path) -> Option<FileStats> {
    let content = fs::read_to_string(path).ok()?;
    let syntax = syn::parse_file(&content).ok()?;

    let mut stats = UnsafeStats::default();
    let mut visitor = UnsafeAnalyzer { stats: &mut stats };
    visitor.visit_file(&syntax);

    Some(FileStats {
        filename: path.display().to_string(),
        stats,
    })
}

fn generate_report(root: &str) -> Report {
    let mut file_reports = Vec::new();
    let mut total = UnsafeStats::default();

    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_entry(|e| {
            e.file_name()
                .to_str()
                .map(|s| s != "target")
                .unwrap_or(true)
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
    {
        let path = entry.path();
        if let Some(file_stats) = analyze_file(path) {
            total.unsafe_fns += file_stats.stats.unsafe_fns;
            total.total_fns += file_stats.stats.total_fns;
            total.unsafe_lines += file_stats.stats.unsafe_lines;

            file_reports.push(file_stats);
        }
    }

    // Strip common root prefix and find max filename length for alignment
    let root_path = std::path::Path::new(&root);
    let mut max_filename_len = 0;
    for file_report in &mut file_reports {
        if let Ok(relative_path) =
            std::path::Path::new(&file_report.filename).strip_prefix(root_path)
        {
            file_report.filename = relative_path.display().to_string();
        }
        max_filename_len = max_filename_len.max(file_report.filename.len());
    }

    file_reports.sort_by(|a, b| a.filename.cmp(&b.filename));

    Report {
        files: file_reports,
        total,
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

fn print_report(report: &Report) {
    let max_filename_len = report
        .files
        .iter()
        .map(|f| f.filename.len())
        .max()
        .unwrap_or_default();

    let max_line_width = report
        .files
        .iter()
        .map(|file_report| {
            format!(
                "{}/{}",
                file_report.stats.unsafe_fns, file_report.stats.total_fns
            )
            .len()
        })
        .max()
        .unwrap_or_default();

    println!(
        "{:<width$} {:>max_line_width$}  lines",
        "",
        "fns",
        width = max_filename_len,
        max_line_width = max_line_width
    );
    for file_report in &report.files {
        let plain_text = format!(
            "{}/{}",
            file_report.stats.unsafe_fns, file_report.stats.total_fns
        );
        let colored_text =
            colorize_percentage(file_report.stats.unsafe_fns, file_report.stats.total_fns);
        println!(
            "{:<width$} {:>max_line_width$} {:>line_width$}",
            file_report.filename,
            colored_text,
            file_report.stats.unsafe_lines,
            width = max_filename_len,
            max_line_width = max_line_width + (colored_text.len() - plain_text.len()),
            line_width = 6,
        );
    }

    println!("== Summary ==");
    println!(
        "Total unsafe functions: {}",
        colorize_percentage(report.total.unsafe_fns, report.total.total_fns)
    );
    println!("Total unsafe lines: {}", report.total.unsafe_lines);
}

fn compare_reports(old_report: &Report, new_report: &Report) {
    let mut old_files: HashMap<String, &FileStats> = HashMap::new();
    let mut new_files: HashMap<String, &FileStats> = HashMap::new();

    for file in &old_report.files {
        old_files.insert(file.filename.clone(), file);
    }
    for file in &new_report.files {
        new_files.insert(file.filename.clone(), file);
    }

    let mut all_files: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    all_files.extend(old_files.keys().cloned());
    all_files.extend(new_files.keys().cloned());

    println!("== Diff Report ==");
    println!("Old report: {}", old_report.timestamp);
    println!("New report: {}", new_report.timestamp);
    println!();

    for filename in all_files {
        let old_stats = old_files.get(&filename);
        let new_stats = new_files.get(&filename);

        match (old_stats, new_stats) {
            (Some(old), Some(new)) => {
                let old_unsafe = old.stats.unsafe_fns;
                let old_total = old.stats.total_fns;
                let old_lines = old.stats.unsafe_lines;
                let new_unsafe = new.stats.unsafe_fns;
                let new_total = new.stats.total_fns;
                let new_lines = new.stats.unsafe_lines;

                if old_unsafe != new_unsafe || old_total != new_total || old_lines != new_lines {
                    let unsafe_diff = new_unsafe as i32 - old_unsafe as i32;
                    let total_diff = new_total as i32 - old_total as i32;
                    let lines_diff = new_lines as i32 - old_lines as i32;

                    println!("{}", filename);

                    let unsafe_color = if unsafe_diff < 0 {
                        "\x1b[32m" // Green for less unsafe
                    } else if unsafe_diff > 0 {
                        "\x1b[31m" // Red for more unsafe
                    } else {
                        ""
                    };

                    let lines_color = if lines_diff < 0 {
                        "\x1b[32m" // Green for fewer unsafe lines
                    } else if lines_diff > 0 {
                        "\x1b[31m" // Red for more unsafe lines
                    } else {
                        ""
                    };

                    println!(
                        "  Unsafe funcs: {} -> {} ({}{}{}{})\x1b[0m",
                        old_unsafe,
                        new_unsafe,
                        unsafe_color,
                        if unsafe_diff >= 0 { "+" } else { "" },
                        unsafe_diff,
                        if unsafe_diff != 0 { "\x1b[0m" } else { "" }
                    );
                    println!(
                        "   Total funcs: {} -> {} ({}{})",
                        old_total,
                        new_total,
                        if total_diff >= 0 { "+" } else { "" },
                        total_diff
                    );
                    println!(
                        "  Unsafe stmts: {} -> {} ({}{}{})\x1b[0m",
                        old_lines,
                        new_lines,
                        lines_color,
                        if lines_diff >= 0 { "+" } else { "" },
                        lines_diff
                    );
                    println!();
                }
            }
            (None, Some(new)) => {
                println!("{} [NEW FILE]", filename);
                println!("  Unsafe funcs: {}", new.stats.unsafe_fns);
                println!("   Total funcs: {}", new.stats.total_fns);
                println!("  Unsafe stmts: {}", new.stats.unsafe_lines);
                println!();
            }
            (Some(old), None) => {
                println!("{} [REMOVED]", filename);
                println!(
                    "  Had {} unsafe / {} total fns, {} unsafe lines",
                    old.stats.unsafe_fns, old.stats.total_fns, old.stats.unsafe_lines
                );
                println!();
            }
            (None, None) => unreachable!(),
        }
    }

    let old_total_unsafe = old_report.total.unsafe_fns;
    let old_total_fns = old_report.total.total_fns;
    let old_total_lines = old_report.total.unsafe_lines;
    let new_total_unsafe = new_report.total.unsafe_fns;
    let new_total_fns = new_report.total.total_fns;
    let new_total_lines = new_report.total.unsafe_lines;

    let unsafe_diff = new_total_unsafe as i32 - old_total_unsafe as i32;
    let total_diff = new_total_fns as i32 - old_total_fns as i32;
    let lines_diff = new_total_lines as i32 - old_total_lines as i32;

    println!("== Summary ==");

    let summary_unsafe_color = if unsafe_diff < 0 {
        "\x1b[32m" // Green for less unsafe
    } else if unsafe_diff > 0 {
        "\x1b[31m" // Red for more unsafe
    } else {
        ""
    };

    let summary_lines_color = if lines_diff < 0 {
        "\x1b[32m" // Green for fewer unsafe lines
    } else if lines_diff > 0 {
        "\x1b[31m" // Red for more unsafe lines
    } else {
        ""
    };

    println!(
        "Unsafe funcs: {} -> {} ({}{}{})\x1b[0m",
        old_total_unsafe,
        new_total_unsafe,
        summary_unsafe_color,
        if unsafe_diff >= 0 { "+" } else { "" },
        unsafe_diff
    );
    println!(
        " Total funcs: {} -> {} ({}{})",
        old_total_fns,
        new_total_fns,
        if total_diff >= 0 { "+" } else { "" },
        total_diff
    );
    println!(
        "Unsafe lines: {} -> {} ({}{}{})\x1b[0m",
        old_total_lines,
        new_total_lines,
        summary_lines_color,
        if lines_diff >= 0 { "+" } else { "" },
        lines_diff
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!(
            r#"
{0} <path-to-crate>                    # Generate report
{0} <path-to-crate> --json <output>    # Export to JSON
{0} --diff <old.json> <new.json>       # Compare reports
"#,
            args[0]
        );
        std::process::exit(1);
    }

    if args.len() >= 4 && args[1] == "--diff" {
        let old_content = fs::read_to_string(&args[2]).expect("Failed to read old report file");
        let new_content = fs::read_to_string(&args[3]).expect("Failed to read new report file");

        let old_report: Report =
            serde_json::from_str(&old_content).expect("Failed to parse old report JSON");
        let new_report: Report =
            serde_json::from_str(&new_content).expect("Failed to parse new report JSON");

        compare_reports(&old_report, &new_report);
        return;
    }

    let root = &args[1];
    let report = generate_report(root);

    if args.len() >= 4 && args[2] == "--json" {
        let mut serializer = serde_json::Serializer::with_formatter(
            Vec::new(),
            serde_json::ser::PrettyFormatter::with_indent(b" "),
        );
        report
            .serialize(&mut serializer)
            .expect("Failed to serialize report");
        let json_output =
            String::from_utf8(serializer.into_inner()).expect("Failed to convert JSON to string");
        fs::write(&args[3], json_output).expect("Failed to write JSON file");
        println!("Report exported to {}", args[3]);
    } else {
        print_report(&report);
    }
}
