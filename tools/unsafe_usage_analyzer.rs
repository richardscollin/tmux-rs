#!/usr/bin/env -S cargo +nightly -Zscript
---
[package]
edition = "2024"
[dependencies]
syn = { version = "2.0.104", features = ["full", "visit"] }
walkdir = "2.5.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
colored = "3.0"
---
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::Path,
};

use colored::{Color, ColoredString, Colorize};
use serde::{Deserialize, Serialize};
use syn::{ExprMethodCall, ExprUnsafe, ItemFn, ItemStatic, StaticMutability, Stmt, visit::Visit};
use walkdir::WalkDir;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
struct CodeStats {
    #[serde(rename = "tf")]
    total_fns: isize,

    #[serde(rename = "ts")]
    total_statements: isize,

    #[serde(rename = "uf")]
    unsafe_fns: isize,

    #[serde(rename = "us")]
    unsafe_statements: isize,

    #[serde(rename = "un")]
    unwraps: isize,

    #[serde(rename = "sm")]
    static_mut_items: isize,
}

impl CodeStats {
    fn should_report_change(&self, rhs: &Self) -> bool {
        let Self {
            total_fns: _,        // ignore
            total_statements: _, // ignore

            unsafe_fns,
            unsafe_statements,
            static_mut_items,
            unwraps,
        } = rhs;

        self.unsafe_fns != *unsafe_fns
            || self.unsafe_statements != *unsafe_statements
            || self.static_mut_items != *static_mut_items
            || self.unwraps != *unwraps
    }
}

impl std::ops::AddAssign for CodeStats {
    fn add_assign(
        &mut self,
        Self {
            total_fns,
            total_statements,

            unsafe_fns,
            unsafe_statements,
            static_mut_items,
            unwraps,
        }: Self,
    ) {
        self.unsafe_fns += unsafe_fns;
        self.unsafe_statements += unsafe_statements;
        self.total_fns += total_fns;
        self.total_statements += total_statements;
        self.static_mut_items += static_mut_items;
        self.unwraps += unwraps;
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct FileStats {
    filename: String,
    stats: CodeStats,
}

#[derive(Clone, Serialize, Deserialize)]
struct Report {
    files: Vec<FileStats>,
    total: CodeStats,
}

struct CodeAnalyzer<'a> {
    stats: &'a mut CodeStats,
}

impl<'a, 'ast> Visit<'ast> for CodeAnalyzer<'a> {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        self.stats.total_fns += 1;
        if i.sig.unsafety.is_some() {
            self.stats.unsafe_fns += 1;
        }
        syn::visit::visit_item_fn(self, i);
    }

    fn visit_expr_unsafe(&mut self, i: &'ast ExprUnsafe) {
        self.stats.unsafe_statements += i.block.stmts.len() as isize;
        syn::visit::visit_expr_unsafe(self, i);
    }

    fn visit_item_static(&mut self, i: &'ast ItemStatic) {
        if !matches!(i.mutability, StaticMutability::None) {
            self.stats.static_mut_items += 1;
        }
        syn::visit::visit_item_static(self, i);
    }

    fn visit_stmt(&mut self, i: &'ast Stmt) {
        self.stats.total_statements += 1;
        syn::visit::visit_stmt(self, i);
    }

    fn visit_expr_method_call(&mut self, i: &'ast ExprMethodCall) {
        if i.method == "unwrap" {
            self.stats.unwraps += 1;
        }
        syn::visit::visit_expr_method_call(self, i);
    }
}

fn analyze_file(path: &Path) -> Option<FileStats> {
    let content = fs::read_to_string(path).ok()?;
    let syntax = syn::parse_file(&content).ok()?;

    let mut stats = CodeStats::default();
    let mut visitor = CodeAnalyzer { stats: &mut stats };
    visitor.visit_file(&syntax);

    Some(FileStats {
        filename: path.display().to_string(),
        stats,
    })
}

fn generate_report(root: &str) -> Report {
    let mut file_reports = Vec::new();
    let mut total = CodeStats::default();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            e.file_name()
                .to_str()
                .map(|s| s != "target" && s != "tools") // skipping our own tool dir
                .unwrap_or(true)
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "rs").unwrap_or(false))
    {
        let path = entry.path();
        if let Some(file_stats) = analyze_file(path) {
            total += file_stats.stats.clone();
            file_reports.push(file_stats);
        }
    }

    // Strip common root prefix and find max filename length for alignment
    let root_path = Path::new(&root);
    let mut max_filename_len = 0;
    for file_report in &mut file_reports {
        if let Ok(relative_path) = Path::new(&file_report.filename).strip_prefix(root_path) {
            file_report.filename = relative_path.display().to_string();
        }
        max_filename_len = max_filename_len.max(file_report.filename.len());
    }

    file_reports.sort_by(|a, b| a.filename.cmp(&b.filename));

    Report {
        files: file_reports,
        total,
    }
}

fn print_report(report: &Report) {
    let mut table = vec![[
        "".into(),
        " (unsafe/total) fns".into(),
        "statements".into(),
        "static mut".into(),
        "unwrap".into(),
    ]];
    table.extend(report.files.iter().map(|file_report| {
        [
            file_report.filename.clone().into(), // filename
            colorize_ratio(file_report.stats.unsafe_fns, file_report.stats.total_fns), // unsafe fns
            format!(
                "{}/{}",
                file_report.stats.unsafe_statements, file_report.stats.total_statements
            )
            .into(), // unsafe statements
            colorize_simple(file_report.stats.static_mut_items), // static mut
            colorize_simple(file_report.stats.unwraps), // unwraps
        ]
    }));

    display_table(&table);
    println!();
    println!("== Summary ==");
    println!(
        "Total unsafe functions: {}",
        colorize_ratio(report.total.unsafe_fns, report.total.total_fns)
    );
    println!(
        "Total statements in unsafe blocks: {}",
        report.total.unsafe_statements
    );
    println!("Total static mut items: {}", report.total.static_mut_items);
    println!("Total unwrap calls: {}", report.total.unwraps);
}

fn display_table(table: &[[ColoredString; 5]]) {
    let mut column_widths = table[0].iter().map(|e| e.len()).collect::<Vec<_>>();

    for row in table {
        for (col_idx, text) in row.iter().enumerate() {
            column_widths[col_idx] = column_widths[col_idx].max(text.len());
        }
    }

    for row in table {
        println!(
            "{:<w0$} {:>w1$} {:>w2$} {:>w3$} {:>w4$}",
            row[0],
            row[1],
            row[2],
            row[3],
            row[4],
            w0 = column_widths[0],
            w1 = column_widths[1],
            w2 = column_widths[2],
            w3 = column_widths[3],
            w4 = column_widths[4],
        )
    }
}

fn print_report_diff(old: &Report, new: &Report) {
    let mut old_files: HashMap<String, &FileStats> = HashMap::new();
    let mut new_files: HashMap<String, &FileStats> = HashMap::new();

    for file in &old.files {
        old_files.insert(file.filename.clone(), file);
    }
    for file in &new.files {
        new_files.insert(file.filename.clone(), file);
    }

    let mut all_files: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    all_files.extend(old_files.keys().cloned());
    all_files.extend(new_files.keys().cloned());

    println!("== Diff Report ==");

    let mut change = false;

    for filename in all_files {
        let old_stats = old_files.get(&filename);
        let new_stats = new_files.get(&filename);

        match (old_stats, new_stats) {
            (Some(old), Some(new)) => {
                if old.stats.should_report_change(&new.stats) {
                    change = true;
                    println!(
                        "{filename}
       Unsafe funcs: {}
        Total funcs: {}
  Unsafe Statements: {}
         static mut: {}
            unwraps: {}
",
                        format_diff(old.stats.unsafe_fns, new.stats.unsafe_fns, DecreaseIs::Good),
                        format_diff(
                            old.stats.total_fns,
                            new.stats.total_fns,
                            DecreaseIs::Neutral
                        ),
                        format_diff(
                            old.stats.unsafe_statements,
                            new.stats.unsafe_statements,
                            DecreaseIs::Good
                        ),
                        format_diff(
                            old.stats.static_mut_items,
                            new.stats.static_mut_items,
                            DecreaseIs::Good
                        ),
                        format_diff(old.stats.unwraps, new.stats.unwraps, DecreaseIs::Good),
                    )
                }
            }
            (None, Some(new)) => {
                println!("{filename} [NEW FILE]");
                println!("  Unsafe funcs: {}", new.stats.unsafe_fns);
                println!("   Total funcs: {}", new.stats.total_fns);
                println!("  Unsafe stmts: {}", new.stats.unsafe_statements);
                println!("       unwraps: {}", new.stats.unwraps);
                println!();
            }
            (Some(old), None) => {
                println!("{filename} [REMOVED]");
                println!(
                    "  Had {} unsafe / {} total fns, {} unsafe lines",
                    old.stats.unsafe_fns, old.stats.total_fns, old.stats.unsafe_statements
                );
                println!();
            }
            (None, None) => unreachable!(),
        }
    }

    if change {
        println!(
            "== Summary ==
Unsafe funcs: {}
Total funcs: {}
Total statements: {}
static mut: {}
unwraps: {}
",
            format_diff(old.total.unsafe_fns, new.total.unsafe_fns, DecreaseIs::Good),
            format_diff(
                old.total.total_fns,
                new.total.total_fns,
                DecreaseIs::Neutral
            ),
            format_diff(
                old.total.unsafe_statements,
                new.total.unsafe_statements,
                DecreaseIs::Good
            ),
            format_diff(
                old.total.static_mut_items,
                new.total.static_mut_items,
                DecreaseIs::Good
            ),
            format_diff(old.total.unwraps, new.total.unwraps, DecreaseIs::Good),
        );
    } else {
        println!("No safety changes.")
    }
}

enum DecreaseIs {
    Neutral,
    Good,
}
fn format_diff(old: isize, new: isize, decrease_is: DecreaseIs) -> String {
    let delta = new - old;
    let plus = if delta > 0 { "+" } else { "" };

    let color = match decrease_is {
        DecreaseIs::Neutral => Color::BrightBlack,
        DecreaseIs::Good => {
            if delta > 0 {
                Color::Red
            } else if delta < 0 {
                Color::Green
            } else {
                Color::BrightBlack
            }
        }
    };

    format!("{old} -> {new} ({plus}{delta})")
        .color(color)
        .to_string()
}

fn colorize_ratio(unsafe_count: isize, total_count: isize) -> ColoredString {
    let color = if total_count == 0 {
        Color::BrightBlack
    } else if unsafe_count == 0 {
        Color::Green
    } else if (unsafe_count as f64 / total_count as f64) < 0.5 {
        Color::Yellow
    } else {
        Color::Red
    };

    format!("{unsafe_count}/{total_count}").color(color)
}

/// colorize such that zero is green, single digit is yellow, more then that is red
fn colorize_simple(count: isize) -> ColoredString {
    let color = if count == 0 {
        Color::Green
    } else if count < 10 {
        Color::Yellow
    } else {
        Color::Red
    };

    count.to_string().color(color)
}

fn main() {
    let flags = flags();
    let flags_with_args = flags_with_args();

    let args = std::env::args().collect::<Vec<String>>();

    if args.len() < 2 || flags.contains("-h") || flags.contains("--help") {
        println!(
            "Usage: ./unsafe_usage_analyzer.rs <crate-root> [--baseline baseline-report.json] [--json report-output.json]"
        );
        return;
    }

    let root = &args[1];
    let report = generate_report(root);

    if let Some(output_file) = flags_with_args.get("--json") {
        let json_report = serde_json::to_string(&report).unwrap();
        std::fs::write(output_file, json_report).unwrap();
    }

    println!("== Code Report ==");
    print_report(&report);

    if let Some(baseline_file) = flags_with_args.get("--baseline") {
        let old_content =
            fs::read_to_string(baseline_file).expect("Failed to read old report file");
        let old_report =
            serde_json::from_str::<Report>(&old_content).expect("Failed to parse old report JSON");

        println!();
        println!();
        print_report_diff(&old_report, &report);
    }
}

pub fn flags() -> HashSet<String> {
    let flags: HashSet<String> = std::env::args()
        .filter(|arg| arg.starts_with('-'))
        .collect();
    flags
}

pub fn flags_with_args() -> HashMap<String, String> {
    let flags_with_args: HashMap<String, String> = std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .filter_map(|e| {
            e[0].starts_with('-')
                .then_some((e[0].clone(), e[1].clone()))
        })
        .collect();
    flags_with_args
}
