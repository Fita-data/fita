use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use fita_core::{MasterRecord, Store};
use fita_store::FileStore;
use serde::Serialize;

use crate::cli::{ExportArgs, ExportFormat, Output};

/// Separator between member keys in the CSV `members` column.
const MEMBER_SEPARATOR: &str = " | ";

#[derive(Serialize)]
struct Report<'a> {
    domain: &'a str,
    records: usize,
    out: &'a Path,
}

pub fn run(args: &ExportArgs, output: Output) -> Result<ExitCode> {
    let project = fita_config::load(&args.project)?;
    let Some(domain) = project.domain(&args.domain) else {
        bail!("the project has no domain called {:?}", args.domain);
    };
    let store = FileStore::new(&args.project);
    if !store.has_master_records(&domain.name) {
        bail!(
            "no golden records for {} yet, run `fita run {} --project {}` first",
            domain.name,
            domain.name,
            args.project.display()
        );
    }
    let records = store.master_records(&domain.name)?;
    let attributes: Vec<&str> = domain.attributes.iter().map(|a| a.name.as_str()).collect();

    match &args.out {
        None => {
            let stdout = io::stdout();
            write(&mut stdout.lock(), args.format, &attributes, &records)?;
        }
        Some(path) => {
            let file =
                File::create(path).with_context(|| format!("cannot create {}", path.display()))?;
            let mut writer = BufWriter::new(file);
            write(&mut writer, args.format, &attributes, &records)?;
            writer
                .flush()
                .with_context(|| format!("cannot write {}", path.display()))?;
            match output {
                Output::Json => super::print_json(&Report {
                    domain: &domain.name,
                    records: records.len(),
                    out: path,
                })?,
                Output::Text => println!(
                    "Wrote {} golden record(s) of {} to {}",
                    records.len(),
                    domain.name,
                    path.display()
                ),
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn write(
    writer: &mut dyn Write,
    format: ExportFormat,
    attributes: &[&str],
    records: &[MasterRecord],
) -> Result<()> {
    match format {
        ExportFormat::Csv => write_csv(writer, attributes, records),
        ExportFormat::Json => {
            serde_json::to_writer_pretty(&mut *writer, records)?;
            writeln!(writer)?;
            Ok(())
        }
    }
}

/// One row per golden record: `master_id`, one column per attribute in the
/// order of the domain, and `members` with the source records.
fn write_csv(writer: &mut dyn Write, attributes: &[&str], records: &[MasterRecord]) -> Result<()> {
    let mut csv = csv::Writer::from_writer(writer);
    let mut header = vec!["master_id"];
    header.extend_from_slice(attributes);
    header.push("members");
    csv.write_record(&header)?;

    for record in records {
        let mut row = vec![record.id.to_string()];
        for attribute in attributes {
            let value = record.values.get(*attribute).map(|v| v.value.clone());
            row.push(value.unwrap_or_default());
        }
        let members: Vec<String> = record.members.iter().map(ToString::to_string).collect();
        row.push(members.join(MEMBER_SEPARATOR));
        csv.write_record(&row)?;
    }
    csv.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use fita_core::{ChosenValue, MasterId, RecordKey, Strategy};

    fn record() -> MasterRecord {
        let billing = RecordKey::new("billing", "1");
        MasterRecord {
            id: MasterId::new("cus", 1).unwrap(),
            domain: "customers".into(),
            members: vec![billing.clone(), RecordKey::new("crm", "C-1")],
            links: Vec::new(),
            values: BTreeMap::from([(
                "legal_name".to_string(),
                ChosenValue {
                    value: "INVENTADA, SL".into(),
                    record: billing,
                    strategy: Strategy::FirstNonEmpty,
                },
            )]),
        }
    }

    #[test]
    fn csv_has_one_column_per_attribute() {
        let mut buffer = Vec::new();
        write_csv(&mut buffer, &["tax_id", "legal_name"], &[record()]).unwrap();
        let text = String::from_utf8(buffer).unwrap();
        assert_eq!(
            text,
            "master_id,tax_id,legal_name,members\n\
             cus_00001,,\"INVENTADA, SL\",billing/1 | crm/C-1\n"
        );
    }

    #[test]
    fn json_keeps_provenance() {
        let mut buffer = Vec::new();
        write(&mut buffer, ExportFormat::Json, &[], &[record()]).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&buffer).unwrap();
        assert_eq!(value[0]["id"], "cus_00001");
        assert_eq!(
            value[0]["values"]["legal_name"]["record"]["source"],
            "billing"
        );
        assert_eq!(
            value[0]["values"]["legal_name"]["strategy"],
            "first_non_empty"
        );
    }
}
