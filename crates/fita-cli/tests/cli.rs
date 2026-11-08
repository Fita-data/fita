//! Runs the real `fita` binary: init --demo, validate, run and export.

use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

/// Companies in the demo. billing has 30 records, crm 25, and 15 of them are
/// the same company in both sources.
const DEMO_COMPANIES: usize = 40;
const DEMO_RECORDS: u64 = 55;
const DEMO_DUPLICATES: u64 = 15;

fn fita(args: &[&str], cwd: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fita"))
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("the fita binary runs")
}

fn ok(args: &[&str], cwd: &Path) -> String {
    let output = fita(args, cwd);
    assert!(
        output.status.success(),
        "fita {args:?} failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn demo_runs_end_to_end() {
    let dir = TempDir::new().unwrap();
    let cwd = dir.path();

    ok(&["init", "my-project", "--demo"], cwd);
    let validate = ok(&["validate", "--project", "my-project"], cwd);
    assert!(validate.contains("is valid"), "{validate}");
    assert!(!validate.contains("warning"), "{validate}");

    let run = ok(
        &[
            "run",
            "customers",
            "--project",
            "my-project",
            "--output",
            "json",
        ],
        cwd,
    );
    let summary: serde_json::Value = serde_json::from_str(&run).unwrap();
    assert_eq!(summary["source_records"], DEMO_RECORDS);
    assert_eq!(summary["master_records"], DEMO_COMPANIES as u64);
    assert_eq!(summary["merged_master_records"], DEMO_DUPLICATES);

    // CSV to a file: a header plus one row per golden record.
    let message = ok(
        &[
            "export",
            "customers",
            "--project",
            "my-project",
            "--format",
            "csv",
            "--out",
            "customers.csv",
        ],
        cwd,
    );
    assert!(message.contains("Wrote 40 golden record(s)"), "{message}");
    let mut reader = csv::Reader::from_path(cwd.join("customers.csv")).unwrap();
    let header = reader.headers().unwrap().clone();
    assert_eq!(
        header.iter().collect::<Vec<_>>(),
        [
            "master_id",
            "tax_id",
            "legal_name",
            "email",
            "phone",
            "postcode",
            "city",
            "members"
        ]
    );
    let rows: Vec<csv::StringRecord> = reader.records().map(Result::unwrap).collect();
    assert_eq!(rows.len(), DEMO_COMPANIES);
    assert_eq!(&rows[0][0], "cus_00001");
    assert_eq!(&rows[DEMO_COMPANIES - 1][0], "cus_00040");

    // JSON to stdout, with the same records.
    let json = ok(
        &[
            "export",
            "customers",
            "--project",
            "my-project",
            "--format",
            "json",
        ],
        cwd,
    );
    let records: Vec<serde_json::Value> = serde_json::from_str(&json).unwrap();
    assert_eq!(records.len(), DEMO_COMPANIES);
    let merged = records
        .iter()
        .filter(|record| record["members"].as_array().unwrap().len() > 1)
        .count();
    assert_eq!(merged as u64, DEMO_DUPLICATES);
}

#[test]
fn export_before_run_explains_what_to_do() {
    let dir = TempDir::new().unwrap();
    ok(&["init", "p", "--demo"], dir.path());
    let output = fita(
        &["export", "customers", "--project", "p", "--format", "csv"],
        dir.path(),
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("run `fita run customers --project p` first"),
        "{stderr}"
    );
}

#[test]
fn invalid_projects_fail_with_the_file() {
    let dir = TempDir::new().unwrap();
    ok(&["init", "p", "--demo"], dir.path());
    std::fs::write(
        dir.path().join("p/rules/customers.yaml"),
        "domain: suppliers\n",
    )
    .unwrap();

    let output = fita(&["validate", "--project", "p"], dir.path());
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("rules/customers.yaml: unknown domain"),
        "{stderr}"
    );

    let output = fita(
        &["validate", "--project", "p", "--output", "json"],
        dir.path(),
    );
    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["valid"], false);

    let output = fita(&["run", "customers", "--project", "p"], dir.path());
    assert!(!output.status.success());
}

#[test]
fn init_refuses_a_folder_with_files() {
    let dir = TempDir::new().unwrap();
    ok(&["init", "p", "--demo"], dir.path());
    let output = fita(&["init", "p", "--demo"], dir.path());
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("is not empty"));
}

#[test]
fn every_command_has_help_with_examples() {
    let dir = TempDir::new().unwrap();
    for command in [
        vec![],
        vec!["init"],
        vec!["validate"],
        vec!["run"],
        vec!["export"],
    ] {
        let mut args = command.clone();
        args.push("--help");
        let help = ok(&args, dir.path());
        assert!(
            help.contains("Examples:"),
            "fita {command:?} --help:\n{help}"
        );
    }
}
