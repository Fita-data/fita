//! Runs a small synthetic project from disk and checks the stored result.
//! Every name, tax id, email and phone here is made up.

use std::fs;
use std::path::Path;

use fita_core::{RecordKey, Store, Strategy};
use fita_store::FileStore;
use tempfile::TempDir;

const FILES: &[(&str, &str)] = &[
    ("fita.yaml", "name: demo\n"),
    (
        "domains/customers.yaml",
        "\
name: customers
prefix: cus
attributes:
  - { name: tax_id, type: tax_id }
  - { name: legal_name, type: company_name }
  - { name: email, type: email, personal: true }
  - { name: phone, type: phone, personal: true }
  - { name: postcode, type: code }
",
    ),
    (
        "sources/billing.yaml",
        "\
name: billing
domain: customers
connector: { type: csv, path: data/billing.csv, separator: ';' }
key: id
modified_at: actualizado
mapping:
  tax_id: nif
  legal_name: razon_social
  email: email
  phone: telefono
  postcode: cp
",
    ),
    (
        "sources/crm.yaml",
        "\
name: crm
domain: customers
connector: { type: csv, path: data/crm.csv }
key: crm_id
modified_at: modified
mapping:
  tax_id: tax
  legal_name: name
  email: mail
  phone: phone
",
    ),
    (
        "rules/customers.yaml",
        "\
domain: customers
version: 1
rules:
  - name: same_tax_id
    when: [equal(tax_id)]
    confidence: 1.0
  - name: same_phone
    when: [equal(phone)]
    confidence: 0.9
  - name: same_email
    when: [equal(email)]
    confidence: 0.8
  - name: similar_name
    when: [similarity(legal_name) >= 0.85]
    action: review
survivorship:
  legal_name: [source(billing)]
  email: [source(crm, billing)]
  phone: [most_recent]
",
    ),
    (
        "data/billing.csv",
        "\
id;razon_social;nif;email;telefono;cp;actualizado
1;Ferretería Inventada, S.L.;ES-B00000001;Facturas@Inventada.example;+34 600 000 001;28001;2026-01-10
2;Panadería Ficticia SA;A00000002;;600 000 002;08001;2026-02-01
3;José Pérez Ejemplo;00000003-T;jose@example.com;;46001;
",
    ),
    (
        "data/crm.csv",
        "\
crm_id,name,tax,mail,phone,modified
C-1,FERRETERIA INVENTADA SL,B-00000001,info@inventada.example,600-000-001,2026-03-05T12:00:00Z
C-2,Panaderia Ficticia,,ventas@ficticia.example,600000002,2026-01-15
C-3,Otra Empresa Falsa SL,B00000009,otra@example.org,,2026-01-01
C-4,Jose Perez,,JOSE@example.com,,
",
    ),
];

fn write_project(dir: &Path) {
    for (path, content) in FILES {
        let full = dir.join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, content).unwrap();
    }
}

#[test]
fn runs_a_project_end_to_end() {
    let dir = TempDir::new().unwrap();
    write_project(dir.path());

    let summary = fita_engine::run(dir.path(), "customers").unwrap();
    assert_eq!(summary.domain, "customers");
    assert_eq!(summary.sources, 2);
    assert_eq!(summary.source_records, 7);
    assert_eq!(summary.rules, 3);
    assert_eq!(summary.links, 3);
    assert_eq!(summary.master_records, 4);
    assert_eq!(summary.merged_master_records, 3);
    assert_eq!(summary.warnings.len(), 1, "{:?}", summary.warnings);
    assert!(summary.warnings[0].contains("similar_name"));

    let store = FileStore::new(dir.path());
    assert_eq!(store.source_records("customers").unwrap().len(), 7);
    assert_eq!(store.normalized_records("customers").unwrap().len(), 7);
    let masters = store.master_records("customers").unwrap();
    let ids: Vec<String> = masters.iter().map(|m| m.id.to_string()).collect();
    assert_eq!(ids, ["cus_00001", "cus_00002", "cus_00003", "cus_00004"]);

    // Same tax id once normalized.
    let ferreteria = &masters[0];
    assert_eq!(
        ferreteria.members,
        vec![RecordKey::new("billing", "1"), RecordKey::new("crm", "C-1")]
    );
    assert_eq!(ferreteria.links[0].rule, "same_tax_id");
    let tax_id = &ferreteria.values["tax_id"];
    assert_eq!(tax_id.value, "B00000001");
    assert_eq!(tax_id.strategy, Strategy::FirstNonEmpty);
    let email = &ferreteria.values["email"];
    assert_eq!(email.value, "info@inventada.example");
    assert_eq!(email.record, RecordKey::new("crm", "C-1"));
    assert_eq!(
        email.strategy,
        Strategy::Source(vec!["crm".into(), "billing".into()])
    );
    let name = &ferreteria.values["legal_name"];
    assert_eq!(name.value, "FERRETERIA INVENTADA SL");
    assert_eq!(name.source(), "billing");
    // The crm record is newer, so its phone wins.
    assert_eq!(
        ferreteria.values["phone"].record,
        RecordKey::new("crm", "C-1")
    );
    assert_eq!(ferreteria.values["phone"].strategy, Strategy::MostRecent);

    // Same phone once normalized. The billing record is newer.
    let panaderia = &masters[1];
    assert_eq!(panaderia.links[0].rule, "same_phone");
    assert_eq!(
        panaderia.values["phone"].record,
        RecordKey::new("billing", "2")
    );
    assert_eq!(panaderia.values["email"].value, "ventas@ficticia.example");

    // Same email once lowercased.
    let jose = &masters[2];
    assert_eq!(
        jose.members,
        vec![RecordKey::new("billing", "3"), RecordKey::new("crm", "C-4")]
    );
    assert_eq!(jose.links[0].rule, "same_email");
    assert_eq!(jose.values["tax_id"].value, "00000003T");

    let alone = &masters[3];
    assert_eq!(alone.members, vec![RecordKey::new("crm", "C-3")]);
    assert!(alone.links.is_empty());
    assert!(!alone.values.contains_key("postcode"));
}

#[test]
fn running_twice_gives_the_same_result() {
    let dir = TempDir::new().unwrap();
    write_project(dir.path());

    fita_engine::run(dir.path(), "customers").unwrap();
    let first = FileStore::new(dir.path())
        .master_records("customers")
        .unwrap();
    fita_engine::run(dir.path(), "customers").unwrap();
    let second = FileStore::new(dir.path())
        .master_records("customers")
        .unwrap();
    assert_eq!(first, second);
}

#[test]
fn invalid_projects_do_not_run() {
    let dir = TempDir::new().unwrap();
    write_project(dir.path());
    fs::write(
        dir.path().join("rules/customers.yaml"),
        "domain: customers\nrules:\n  - name: x\n    when: [equal(nope)]\n",
    )
    .unwrap();

    let error = fita_engine::run(dir.path(), "customers").unwrap_err();
    assert!(
        error.to_string().contains("rules/customers.yaml"),
        "{error}"
    );
    assert!(!dir.path().join(".fita").exists());
}
