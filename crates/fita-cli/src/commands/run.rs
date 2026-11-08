use std::process::ExitCode;

use anyhow::Result;
use fita_store::STATE_DIR;

use crate::cli::{Output, RunArgs};

pub fn run(args: &RunArgs, output: Output) -> Result<ExitCode> {
    let summary = fita_engine::run(&args.project, &args.domain)?;
    match output {
        Output::Json => super::print_json(&summary)?,
        Output::Text => {
            println!(
                "Ran {}: {} source(s), {} record(s), {} rule(s)",
                summary.domain, summary.sources, summary.source_records, summary.rules
            );
            println!(
                "Found {} link(s) and built {} golden record(s), {} of them from more than one record",
                summary.links, summary.master_records, summary.merged_master_records
            );
            println!("Saved in {}", args.project.join(STATE_DIR).display());
            for warning in &summary.warnings {
                println!("warning: {warning}");
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
