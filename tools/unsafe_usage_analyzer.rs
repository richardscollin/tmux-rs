#!/usr/bin/env -S cargo +nightly -Zscript
---cargo
[package]
edition = "2024"
[dependencies]
colored = { version = "3.0",     features = [] }
csv     = { version = "1.3.1",   features = [] }
serde   = { version = "1.0",     features = ["derive"] }
syn     = { version = "2.0.104", features = ["full", "visit"] }
walkdir = { version = "2.5.0",   features = [] }
---
use ::std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt::Write,
    fs,
    path::Path,
};
use colored::{Color, ColoredString, Colorize};
use syn::{ExprMethodCall, ExprUnsafe, ItemFn, ItemStatic, StaticMutability, Stmt, visit::Visit};
use walkdir::WalkDir;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct CodeStats {
    filename: String,
    total_fns: isize,
    total_statements: isize,
    unsafe_fns: isize,
    unsafe_statements: isize,
    unwraps: isize,
    static_mut_items: isize,
}

#[derive(Clone)]
struct Report {
    total: CodeStats,
    files: BTreeMap<String, CodeStats>,
}

impl CodeStats {
    fn new(filename: String) -> Self {
        Self {
            filename,
            total_fns: 0,
            total_statements: 0,
            unsafe_fns: 0,
            unsafe_statements: 0,
            unwraps: 0,
            static_mut_items: 0,
        }
    }

    fn should_report_change(&self, rhs: &Self) -> bool {
        let Self {
            filename: _,
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

use std::iter::Iterator;
fn calc_total<'a>(stats: impl Iterator<Item = &'a CodeStats>) -> CodeStats {
    let mut unsafe_fns = 0;
    let mut unsafe_statements = 0;
    let mut total_fns = 0;
    let mut total_statements = 0;
    let mut static_mut_items = 0;
    let mut unwraps = 0;

    for e in stats {
        unsafe_fns += e.unsafe_fns;
        unsafe_statements += e.unsafe_statements;
        total_fns += e.total_fns;
        total_statements += e.total_statements;
        static_mut_items += e.static_mut_items;
        unwraps += e.unwraps;
    }
    CodeStats {
        filename: "total".into(),
        unsafe_fns,
        unsafe_statements,
        total_fns,
        total_statements,
        static_mut_items,
        unwraps,
    }
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

fn analyze_file(path: &Path) -> Option<CodeStats> {
    let content = fs::read_to_string(path).ok()?;
    let syntax = syn::parse_file(&content).ok()?;

    let mut stats = CodeStats::new(path.display().to_string());
    let mut visitor = CodeAnalyzer { stats: &mut stats };
    visitor.visit_file(&syntax);

    Some(stats)
}

fn generate_report(root: &str) -> Report {
    let mut file_reports = BTreeMap::new();

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
        if let Some(mut stats) = analyze_file(path) {
            let root_path = Path::new(root);
            if let Ok(relative_path) = path.strip_prefix(root_path) {
                stats.filename = relative_path.display().to_string();
                file_reports.insert(relative_path.display().to_string(), stats);
            } else {
                file_reports.insert(path.display().to_string(), stats);
            }
        }
    }

    Report {
        total: calc_total(file_reports.values()),
        files: file_reports,
    }
}

fn format_markdown_report(report: &Report) -> String {
    let mut buf = "| (unsafe/total) | fns | statements | static mut | unwrap |\n".to_string();
    buf.push_str("| -- | -: | -: | -: | -: |\n");
    for (_, file_report) in &report.files {
        buf += &format!(
            "|{}|{}|{}|{}|{}|\n",
            file_report.filename,
            format!("{}/{}", file_report.unsafe_fns, file_report.total_fns),
            format!(
                "{}/{}",
                file_report.unsafe_statements, file_report.total_statements
            ),
            file_report.static_mut_items, // static mut
            file_report.unwraps
        );
    }
    buf
}

fn print_report(report: &Report) {
    let mut table = vec![[
        "".into(),
        " (unsafe/total) fns".into(),
        "statements".into(),
        "static mut".into(),
        "unwrap".into(),
    ]];
    table.extend(report.files.values().map(|file_report| {
        [
            file_report.filename.clone().into(), // filename
            colorize_ratio(file_report.unsafe_fns, file_report.total_fns), // unsafe fns
            format!(
                "{}/{}",
                file_report.unsafe_statements, file_report.total_statements
            )
            .into(), // unsafe statements
            colorize_simple(file_report.static_mut_items), // static mut
            colorize_simple(file_report.unwraps), // unwraps
        ]
    }));

    display_table(&table);
    println!();
    println!("== Summary ==");
    println!(
        "Total unsafe functions: {}",
        colorize_percentage(report.total.unsafe_fns, report.total.total_fns)
    );
    println!(
        "Total statements in unsafe blocks: {}",
        report.total.unsafe_statements
    );
    println!("Total static mut items: {}", report.total.static_mut_items);
    println!("Total unwrap calls: {}", report.total.unwraps);
}

fn display_table<const N: usize>(table: &[[ColoredString; N]]) {
    let mut column_widths = vec![0; N];
    for row in table {
        for (c, text) in row.iter().enumerate() {
            column_widths[c] = column_widths[c].max(text.len());
        }
    }

    for row in table {
        let mut out = String::new();
        let mut it = row.iter().zip(&column_widths);

        // left align first column
        let (col, width) = it.next().unwrap();
        _ = write!(&mut out, "{col:<width$}");

        // right align other columns
        for (col, width) in it {
            _ = write!(&mut out, " {col:>width$}");
        }
        println!("{out}")
    }
}

fn print_report_diff(old: &Report, new: &Report) {
    let old_files = &old.files;
    let new_files = &new.files;

    let mut all_files: std::collections::BTreeSet<&String> = std::collections::BTreeSet::new();

    all_files.extend(old_files.keys());
    all_files.extend(new_files.keys());

    println!("== Diff Report ==");

    let mut change = false;

    for filename in all_files {
        match (old_files.get(filename), new_files.get(filename)) {
            (Some(old), Some(new)) => {
                if old.should_report_change(new) {
                    change = true;
                    println!(
                        "{filename}
       Unsafe funcs: {}
        Total funcs: {}
  Unsafe Statements: {}
         static mut: {}
            unwraps: {}
",
                        format_diff(old.unsafe_fns, new.unsafe_fns, DecreaseIs::Good),
                        format_diff(old.total_fns, new.total_fns, DecreaseIs::Neutral),
                        format_diff(
                            old.unsafe_statements,
                            new.unsafe_statements,
                            DecreaseIs::Good
                        ),
                        format_diff(old.static_mut_items, new.static_mut_items, DecreaseIs::Good),
                        format_diff(old.unwraps, new.unwraps, DecreaseIs::Good),
                    )
                }
            }
            (None, Some(new)) => {
                println!("{filename} [NEW FILE]");
                println!("  Unsafe funcs: {}", new.unsafe_fns);
                println!("   Total funcs: {}", new.total_fns);
                println!("  Unsafe stmts: {}", new.unsafe_statements);
                println!("       unwraps: {}", new.unwraps);
                println!();
            }
            (Some(old), None) => {
                println!("{filename} [REMOVED]");
                println!(
                    "  Had {} unsafe / {} total fns, {} unsafe lines",
                    old.unsafe_fns, old.total_fns, old.unsafe_statements
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

fn colorize_percentage(unsafe_count: isize, total_count: isize) -> ColoredString {
    let color = if total_count == 0 {
        Color::BrightBlack
    } else if unsafe_count == 0 {
        Color::Green
    } else if (unsafe_count as f64 / total_count as f64) < 0.5 {
        Color::Yellow
    } else {
        Color::Red
    };

    let percentage = if total_count == 0 {
        0.0
    } else {
        (unsafe_count as f64 / total_count as f64) * 100.0
    };

    format!("{percentage:.02}% ({unsafe_count} / {total_count})").color(color)
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

    if let Some(output_format) = flags_with_args.get("--output") {
        if output_format == "md" {
            println!("{}", format_markdown_report(&report));
            std::process::exit(0);
        }
    }

    if let Some(output_file) = flags_with_args.get("--csv") {
        let mut writer = csv::WriterBuilder::new().from_path(output_file).unwrap();
        for record in report.files.values() {
            writer.serialize(record).unwrap();
        }
    }

    println!("== Code Report ==");
    print_report(&report);

    if let Some(baseline_file) = flags_with_args.get("--baseline") {
        let mut reader = csv::Reader::from_path(baseline_file).unwrap();

        let files = reader
            .records()
            .map(|result| {
                let record = result.unwrap();
                let row: CodeStats = record.deserialize(None).unwrap();
                (row.filename.clone(), row)
            })
            .collect::<BTreeMap<String, CodeStats>>();
        let old_report = Report {
            total: calc_total(files.values()),
            files,
        };

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
