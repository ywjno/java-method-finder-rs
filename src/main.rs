use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use cafebabe::{attributes::AttributeData, bytecode::Opcode, parse_class};
use clap::{Parser, ValueEnum};
use log::{debug, error, LevelFilter};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::Serialize;
use simple_logger::SimpleLogger;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(name = "jmf", about = "Java Method Finder", long_about = None)]
struct Args {
    #[arg(short = 'c', long = "class")]
    target_class: String,

    #[arg(short = 'm', long = "method")]
    target_method: String,

    #[arg(short = 's', long = "scan", default_value = "./target/classes")]
    scan_folder: String,

    #[arg(short = 'f', long = "format", value_enum, default_value_t = Formatter::Txt)]
    format: Formatter,

    #[arg(short = 'v', long = "verbose")]
    verbose: bool,
}

#[derive(Debug, Copy, Clone, ValueEnum)]
enum Formatter {
    #[value(name = "txt")]
    Txt,
    #[value(name = "json")]
    Json,
}

#[derive(Debug, Serialize, Clone)]
struct FoundCall {
    class_name: String,
    method_name: String,
    line_number: u16,
}

impl FoundCall {
    pub fn new(class_name: String, method_name: String, line_number: u16) -> Self {
        Self {
            class_name,
            method_name,
            line_number,
        }
    }
}

impl std::fmt::Display for FoundCall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}#{} (L{})",
            self.class_name.replace('/', "."),
            self.method_name,
            self.line_number
        )
    }
}

#[derive(Debug, Serialize)]
struct SearchResult {
    target: String,
    calls: Vec<FoundCall>,
}

impl SearchResult {
    pub fn new(target_class: &str, target_method: &str, calls: Vec<FoundCall>) -> Self {
        Self {
            target: format!("{}#{}", target_class, target_method),
            calls,
        }
    }

    pub fn to_text(&self) -> String {
        let mut output = vec![self.target.clone()];
        if self.calls.is_empty() {
            output.push("No results".to_string());
        } else {
            output.extend(self.calls.iter().map(|call| format!(" - {}", call)));
        }
        output.join("\n")
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self).unwrap()
    }
}

struct MethodFinder {
    args: Args,
}

fn init_logger(verbose: bool) {
    SimpleLogger::new()
        .with_level(if verbose { LevelFilter::Debug } else { LevelFilter::Info })
        .without_timestamps()
        .with_module_level("simple_logger", LevelFilter::Error)
        .init()
        .unwrap();
}

impl MethodFinder {
    fn new(args: Args) -> Self {
        init_logger(args.verbose);
        MethodFinder { args }
    }

    fn log_debug(&self, message: &str) {
        if self.args.verbose {
            debug!("{}", message);
        }
    }

    fn scan_folder(&self) -> Result<Vec<FoundCall>> {
        let scan_path = PathBuf::from(&self.args.scan_folder);
        if !scan_path.exists() {
            return Err(anyhow::anyhow!("Scan folder does not exist: {}", scan_path.display()));
        }
        if !scan_path.is_dir() {
            return Err(anyhow::anyhow!("Scan path is not a directory: {}", scan_path.display()));
        }
        self.log_debug(&format!("Start scanning folder: {}", scan_path.display()));

        let class_files: Vec<_> = WalkDir::new(&scan_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file() && e.path().extension().map_or(false, |ext| ext == "class"))
            .map(|e| e.path().to_owned())
            .collect();

        let results: Vec<FoundCall> = class_files
            .par_iter()
            .filter_map(|path| {
                self.log_debug(&format!("Analyzing class file: {}", path.display()));
                match self.analyze_class(path) {
                    Ok(found_calls) => {
                        if !found_calls.is_empty() {
                            Some(found_calls)
                        } else {
                            None
                        }
                    }
                    Err(e) => {
                        error!("Error analyzing {}: {:#}", path.display(), e);
                        None
                    }
                }
            })
            .flatten()
            .collect();

        Ok(results)
    }

    fn analyze_class(&self, path: &Path) -> Result<Vec<FoundCall>> {
        let mut found_calls = Vec::new();
        let class_data = fs::read(path).with_context(|| format!("Failed to read class file {}", path.display()))?;
        let class_file =
            parse_class(&class_data).with_context(|| format!("Failed to parse class file {}", path.display()))?;
        let target_class = self.args.target_class.replace('.', "/");

        let class_name = class_file.this_class;

        // Skip if this is the target class
        if class_name == target_class {
            return Ok(found_calls);
        }

        self.log_debug(&format!("Visiting class: {}", class_name));

        for method in &class_file.methods {
            let method_name = &method.name;

            let code_attr = method
                .attributes
                .iter()
                .find_map(|attr| {
                    if let AttributeData::Code(code) = &attr.data {
                        Some(code)
                    } else {
                        None
                    }
                })
                .with_context(|| format!("Code attribute not found in method {}#{}", class_name, method_name))?;

            let line_number_table = code_attr
                .attributes
                .iter()
                .find_map(|attr| {
                    if let AttributeData::LineNumberTable(lnt) = &attr.data {
                        Some(lnt)
                    } else {
                        None
                    }
                })
                .with_context(|| format!("LineNumberTable not found in method {}#{}", class_name, method_name))?;

            if let Some(bytecode) = &code_attr.bytecode {
                self.log_debug(&format!("Visiting method: {}#{}", class_name, method_name));

                for opcode in &bytecode.opcodes {
                    if let Opcode::Invokespecial(member_ref)
                    | Opcode::Invokestatic(member_ref)
                    | Opcode::Invokevirtual(member_ref) = &opcode.1
                    {
                        let offset = &opcode.0;

                        let index = line_number_table.partition_point(|entry| entry.start_pc <= *offset as u16);

                        if index > 0
                            && member_ref.class_name == target_class
                            && member_ref.name_and_type.name == self.args.target_method
                        {
                            let line_number = line_number_table[index - 1].line_number;
                            let found_call =
                                FoundCall::new(class_name.to_string(), method_name.to_string(), line_number);
                            found_calls.push(found_call.clone());
                            self.log_debug(&format!("Found method call: {}", found_call));
                        }
                    }
                }
            } else {
                anyhow::bail!("No bytecode found in method {}#{}", class_name, method_name);
            }
        }

        Ok(found_calls)
    }

    fn print_results(&self, results: &[FoundCall]) {
        let search_result = SearchResult::new(
            &self.args.target_class,
            &self.args.target_method,
            results
                .iter()
                .map(|r| FoundCall::new(r.class_name.clone(), r.method_name.clone(), r.line_number))
                .collect(),
        );
        if results.is_empty() {
            println!("{}#{}", self.args.target_class, self.args.target_method);
            println!("No results");
        } else {
            match self.args.format {
                Formatter::Txt => {
                    println!("{}", search_result.to_text());
                }
                Formatter::Json => {
                    println!("{}", search_result.to_json());
                }
            }
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let finder = MethodFinder::new(args);

    match finder.scan_folder() {
        Ok(results) => {
            finder.print_results(&results);
            Ok(())
        }
        Err(e) => {
            eprintln!("Error: {:#}", e);
            Err(e)
        }
    }
}
