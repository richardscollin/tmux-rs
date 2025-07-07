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
use std::{fs, path::Path};

use syn::{ItemFn, visit::Visit};
use walkdir::WalkDir;

#[derive(Clone, Default, Debug)]
struct FileStats {
    filename: String,
    stats: CodeStats,
}

#[derive(Clone, Default, Debug)]
struct CodeStats {
    candidates: Vec<String>,
}

struct CodeAnalyzer<'a> {
    stats: &'a mut CodeStats,
}

impl<'a, 'ast> Visit<'ast> for CodeAnalyzer<'a> {
    fn visit_item_fn(&mut self, i: &'ast ItemFn) {
        if i.sig.unsafety.is_some() {
            // if function is unsafe and it has no raw pointer arguments add it to the list
            let has_raw_pointer = i.sig.inputs.iter().any(|arg| match arg {
                syn::FnArg::Typed(pat_type) => {
                    matches!(*pat_type.ty, syn::Type::Ptr(_))
                }
                _ => false,
            });

            if !has_raw_pointer {
                self.stats.candidates.push(i.sig.ident.to_string())
            }
        }
        syn::visit::visit_item_fn(self, i);
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

fn find_candidates(root: &str) -> Vec<FileStats> {
    let mut file_reports = Vec::new();

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
    file_reports.retain(|r| !r.stats.candidates.is_empty());
    file_reports
}

// find good candidates for functions to convert to being safe
// this is very simplistic, the heuristic is if the function
// has no raw pointers as parameters, it may be a good candidate
//
// there may be other reasons why one of these functions can't be converted

fn main() {
    let args = std::env::args().collect::<Vec<String>>();
    let root = &args[1];
    let report = find_candidates(root);

    for rep in report {
        println!("{}", rep.filename);
        for candidate in rep.stats.candidates {
            println!("\t{candidate}");
        }
        println!();
    }
}
