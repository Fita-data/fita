use std::fs;

use fita_core::{RecordKey, Store};
use fita_store::MemoryStore;
use tempfile::TempDir;

use crate::{run_with_store, EngineError};

const SETTINGS: &str = "name: test\n";
const DOMAIN: &str = "\
name: suppliers
prefix: sup
attributes:
  - { name: tax_id, type: tax_id }
  - { name: name, type: company_name }
";
const SOURCE: &str = "\
name: erp
domain: suppliers
connector: { type: csv, path: erp.csv }
key: id
mapping: { tax_id: cif, name: nombre }
";
const CSV: &str = "\
id,cif,nombre
1,B00000001,Suministros Inventados SL
2,b-00000001,\"Suministros Inventados, S.L.\"
3,B00000002,Transportes Ficticios SA
";
const RULES: &str = "\
domain: suppliers
rules:
  - name: same_tax_id
    when: [equal(tax_id)]
";

fn project(files: &[(&str, &str)]) -> (TempDir, fita_config::Project) {
    let dir = TempDir::new().unwrap();
    for (path, content) in files {
        let full = dir.path().join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, content).unwrap();
    }
    let project = fita_config::load(dir.path()).unwrap();
    (dir, project)
}

fn base_files() -> Vec<(&'static str, &'static str)> {
    vec![
        ("fita.yaml", SETTINGS),
        ("domains/suppliers.yaml", DOMAIN),
        ("sources/erp.yaml", SOURCE),
        ("erp.csv", CSV),
    ]
}

#[test]
fn runs_and_saves_into_the_store() {
    let mut files = base_files();
    files.push(("rules/suppliers.yaml", RULES));
    let (_dir, project) = project(&files);
    let mut store = MemoryStore::new();

    let summary = run_with_store(&project, "suppliers", &mut store).unwrap();
    assert_eq!(summary.sources, 1);
    assert_eq!(summary.source_records, 3);
    assert_eq!(summary.rules, 1);
    assert_eq!(summary.links, 1);
    assert_eq!(summary.master_records, 2);
    assert_eq!(summary.merged_master_records, 1);

    assert_eq!(store.source_records("suppliers").unwrap().len(), 3);
    let normalized = store.normalized_records("suppliers").unwrap();
    assert_eq!(normalized[1].value("tax_id"), Some("B00000001"));
    assert_eq!(
        normalized[1].value("name"),
        Some("SUMINISTROS INVENTADOS SL")
    );

    let masters = store.master_records("suppliers").unwrap();
    assert_eq!(masters[0].id.to_string(), "sup_00001");
    assert_eq!(
        masters[0].members,
        vec![RecordKey::new("erp", "1"), RecordKey::new("erp", "2")]
    );
    assert_eq!(masters[0].values["name"].value, "SUMINISTROS INVENTADOS SL");
    assert_eq!(masters[1].id.to_string(), "sup_00002");
    assert_eq!(masters[1].members, vec![RecordKey::new("erp", "3")]);
}

#[test]
fn without_rules_every_record_is_its_own_master() {
    let (_dir, project) = project(&base_files());
    let mut store = MemoryStore::new();
    let summary = run_with_store(&project, "suppliers", &mut store).unwrap();
    assert_eq!(summary.rules, 0);
    assert_eq!(summary.master_records, 3);
    assert_eq!(summary.merged_master_records, 0);
}

#[test]
fn rejects_unknown_domains() {
    let (_dir, project) = project(&base_files());
    let error = run_with_store(&project, "customers", &mut MemoryStore::new()).unwrap_err();
    assert!(matches!(error, EngineError::UnknownDomain(name) if name == "customers"));
}

#[test]
fn source_errors_name_the_source_and_store_nothing() {
    let mut files = base_files();
    files.retain(|(path, _)| *path != "erp.csv");
    files.push(("erp.csv", "id,nombre\n1,Sin CIF SL\n"));
    let (_dir, project) = project(&files);
    let mut store = MemoryStore::new();

    let error = run_with_store(&project, "suppliers", &mut store).unwrap_err();
    assert!(error.to_string().starts_with("source \"erp\":"), "{error}");
    assert!(error.to_string().contains("\"cif\""), "{error}");
    assert!(store.source_records("suppliers").unwrap().is_empty());
}

#[test]
fn a_domain_without_sources_produces_nothing() {
    let (_dir, project) = project(&[("fita.yaml", SETTINGS), ("domains/suppliers.yaml", DOMAIN)]);
    let summary = run_with_store(&project, "suppliers", &mut MemoryStore::new()).unwrap();
    assert_eq!(summary.sources, 0);
    assert_eq!(summary.master_records, 0);
}
