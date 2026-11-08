use std::process::ExitCode;

use anyhow::Result;
use fita_config::{Issue, LoadError};
use serde::Serialize;

use crate::cli::{Output, ValidateArgs};

#[derive(Serialize)]
struct Report {
    valid: bool,
    domains: usize,
    sources: usize,
    errors: Vec<String>,
    warnings: Vec<String>,
}

pub fn run(args: &ValidateArgs, output: Output) -> Result<ExitCode> {
    match fita_config::load(&args.project) {
        Ok(project) => {
            let warnings = texts(&project.warnings);
            match output {
                Output::Json => super::print_json(&Report {
                    valid: true,
                    domains: project.domains.len(),
                    sources: project.sources.len(),
                    errors: Vec::new(),
                    warnings,
                })?,
                Output::Text => {
                    println!(
                        "Project {} is valid: {} domain(s), {} source(s)",
                        project.name,
                        project.domains.len(),
                        project.sources.len()
                    );
                    for warning in warnings {
                        println!("warning: {warning}");
                    }
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Err(LoadError::Invalid(issues)) if output == Output::Json => {
            super::print_json(&Report {
                valid: false,
                domains: 0,
                sources: 0,
                errors: texts(&issues),
                warnings: Vec::new(),
            })?;
            Ok(ExitCode::FAILURE)
        }
        Err(error) => Err(error.into()),
    }
}

fn texts(issues: &[Issue]) -> Vec<String> {
    issues.iter().map(ToString::to_string).collect()
}
