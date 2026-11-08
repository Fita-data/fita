use std::fs;

use fita_core::Strategy;
use fita_normalize::AttributeType;
use tempfile::TempDir;

use crate::{load, Connector, LoadError, MatchRule, Project};

const SETTINGS: &str = "name: demo\n";

const CUSTOMERS: &str = "\
name: customers
prefix: cus
attributes:
  - { name: tax_id, type: tax_id }
  - { name: legal_name, type: company_name }
  - { name: email, type: email, personal: true }
  - { name: postcode, type: code }
";

const BILLING: &str = "\
name: billing
domain: customers
connector:
  type: csv
  path: data/billing.csv
  separator: ';'
key: id
modified_at: updated
mapping:
  tax_id: nif
  legal_name: name
";

const CRM: &str = "\
name: crm
domain: customers
connector: { type: csv, path: data/crm.csv }
key: crm_id
mapping:
  email: mail
";

const RULES: &str = "\
domain: customers
version: 3
rules:
  - name: same_tax_id_and_postcode
    when: [equal(tax_id), equal(postcode)]
    confidence: 0.95
  - name: same_email
    when: [equal(email)]
survivorship:
  tax_id: [source(billing), most_recent]
  email: [source(crm, billing)]
";

fn project(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    for (path, content) in files {
        let full = dir.path().join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, content).unwrap();
    }
    dir
}

fn valid_files() -> Vec<(&'static str, &'static str)> {
    vec![
        ("fita.yaml", SETTINGS),
        ("domains/customers.yaml", CUSTOMERS),
        ("sources/billing.yaml", BILLING),
        ("sources/crm.yml", CRM),
        ("rules/customers.yaml", RULES),
    ]
}

/// Valid files with one of them replaced.
fn with(path: &'static str, content: &'static str) -> Vec<(&'static str, &'static str)> {
    let mut files = valid_files();
    files.retain(|(p, _)| *p != path);
    files.push((path, content));
    files
}

fn load_ok(files: &[(&str, &str)]) -> Project {
    let dir = project(files);
    load(dir.path()).unwrap_or_else(|error| panic!("{error}"))
}

/// Loads a project that must fail and returns its errors as text.
fn load_errors(files: &[(&str, &str)]) -> Vec<String> {
    let dir = project(files);
    match load(dir.path()) {
        Err(LoadError::Invalid(issues)) => issues.iter().map(ToString::to_string).collect(),
        other => panic!("expected validation errors, got {other:?}"),
    }
}

fn assert_has(messages: &[String], expected: &str) {
    assert!(
        messages.iter().any(|m| m.contains(expected)),
        "no message contains {expected:?}: {messages:#?}"
    );
}

#[test]
fn loads_a_valid_project() {
    let dir = project(&valid_files());
    let project = load(dir.path()).unwrap();

    assert_eq!(project.name, "demo");
    assert!(project.warnings.is_empty(), "{:?}", project.warnings);

    let customers = project.domain("customers").unwrap();
    assert_eq!(customers.prefix, "cus");
    assert_eq!(customers.attributes.len(), 4);
    assert_eq!(
        customers.attribute_types()["legal_name"],
        AttributeType::CompanyName
    );
    assert!(customers.attribute("email").unwrap().personal);

    let sources = project.sources_of("customers");
    assert_eq!(sources.len(), 2);
    assert_eq!(sources[0].name, "billing");
    assert_eq!(sources[0].modified_at.as_deref(), Some("updated"));
    assert_eq!(
        sources[0].connector,
        Connector::Csv {
            path: dir.path().join("data/billing.csv"),
            separator: b';'
        }
    );
    assert_eq!(
        sources[1].connector,
        Connector::Csv {
            path: dir.path().join("data/crm.csv"),
            separator: b','
        }
    );

    let rules = project.rules_of("customers").unwrap();
    assert_eq!(rules.version, Some(3));
    assert_eq!(
        rules.rules,
        vec![
            MatchRule {
                name: "same_tax_id_and_postcode".into(),
                attributes: vec!["tax_id".into(), "postcode".into()],
                confidence: 0.95,
            },
            MatchRule {
                name: "same_email".into(),
                attributes: vec!["email".into()],
                confidence: 1.0,
            },
        ]
    );
    assert_eq!(
        rules.survivorship["tax_id"],
        vec![
            Strategy::Source(vec!["billing".into()]),
            Strategy::MostRecent
        ]
    );
}

#[test]
fn rules_and_sources_are_optional() {
    let project = load_ok(&[
        ("fita.yaml", SETTINGS),
        ("domains/customers.yaml", CUSTOMERS),
    ]);
    assert!(project.sources.is_empty());
    assert!(project.rules_of("customers").is_none());
}

#[test]
fn missing_settings_is_not_a_project() {
    let dir = project(&[("domains/customers.yaml", CUSTOMERS)]);
    assert!(matches!(load(dir.path()), Err(LoadError::NotAProject(_))));
}

#[test]
fn needs_at_least_one_domain() {
    let errors = load_errors(&[("fita.yaml", SETTINGS)]);
    assert_has(&errors, "domains: the project has no domains");
}

#[test]
fn reports_yaml_errors_with_the_file() {
    let errors = load_errors(&with("sources/crm.yml", "name: [unclosed\n"));
    assert_has(&errors, "sources/crm.yml: invalid YAML");
}

#[test]
fn rejects_unknown_keys() {
    let errors = load_errors(&with("fita.yaml", "name: demo\nnmae: typo\n"));
    assert_has(&errors, "fita.yaml: invalid YAML: unknown field `nmae`");
}

#[test]
fn rejects_unknown_attribute_types() {
    let domain = "name: customers\nprefix: cus\nattributes:\n  - { name: tax_id, type: nif }\n";
    let errors = load_errors(&[("fita.yaml", SETTINGS), ("domains/customers.yaml", domain)]);
    assert_has(
        &errors,
        "domains/customers.yaml: attribute \"tax_id\": unknown attribute type \"nif\"",
    );
}

#[test]
fn rejects_duplicate_attributes() {
    let domain = "name: customers\nprefix: cus\nattributes:\n  - { name: a, type: text }\n  - { name: a, type: code }\n";
    let errors = load_errors(&[("fita.yaml", SETTINGS), ("domains/customers.yaml", domain)]);
    assert_has(&errors, "attribute \"a\" is defined more than once");
}

#[test]
fn rejects_repeated_and_invalid_prefixes() {
    let suppliers =
        "name: suppliers\nprefix: cus\nattributes:\n  - { name: tax_id, type: tax_id }\n";
    let errors = load_errors(&with("domains/suppliers.yaml", suppliers));
    assert_has(
        &errors,
        "domains/suppliers.yaml: prefix \"cus\" is used by another domain",
    );

    let bad = "name: suppliers\nprefix: Sup_\nattributes:\n  - { name: tax_id, type: tax_id }\n";
    let errors = load_errors(&with("domains/suppliers.yaml", bad));
    assert_has(&errors, "invalid prefix \"Sup_\"");
}

#[test]
fn rejects_repeated_domain_names() {
    let copy = "name: customers\nprefix: cu2\nattributes:\n  - { name: tax_id, type: tax_id }\n";
    let errors = load_errors(&with("domains/copy.yaml", copy));
    assert_has(&errors, "domain \"customers\" is defined more than once");
}

#[test]
fn rejects_bad_sources() {
    let source = "\
name: erp
domain: suppliers
connector: { type: postgres, path: x }
key: id
mapping: { tax_id: nif }
";
    let errors = load_errors(&with("sources/erp.yaml", source));
    assert_has(&errors, "sources/erp.yaml: unknown domain \"suppliers\"");
    assert_has(&errors, "connector type \"postgres\" is not supported yet");

    let source = "\
name: erp
domain: customers
connector: { type: csv, path: erp.csv, separator: ';;' }
key: id
mapping: { vat: nif }
";
    let errors = load_errors(&with("sources/erp.yaml", source));
    assert_has(
        &errors,
        "attribute \"vat\" does not exist in domain \"customers\"",
    );
    assert_has(&errors, "separator \";;\" must be a single ASCII character");
}

#[test]
fn rejects_repeated_source_names() {
    let errors = load_errors(&with("sources/billing2.yaml", BILLING));
    assert_has(
        &errors,
        "sources/billing2.yaml: source \"billing\" is defined more than once",
    );
}

#[test]
fn rejects_confidence_out_of_range() {
    let rules = "\
domain: customers
rules:
  - name: too_sure
    when: [equal(tax_id)]
    confidence: 1.5
";
    let errors = load_errors(&with("rules/customers.yaml", rules));
    assert_has(
        &errors,
        "rules/customers.yaml: rule \"too_sure\": confidence must be between 0 and 1",
    );
}

#[test]
fn rejects_rules_on_unknown_attributes_and_functions() {
    let rules = "\
domain: customers
rules:
  - name: by_vat
    when: [equal(vat)]
  - name: made_up
    when: [same(tax_id)]
";
    let errors = load_errors(&with("rules/customers.yaml", rules));
    assert_has(&errors, "rule \"by_vat\": attribute \"vat\" does not exist");
    assert_has(&errors, "rule \"made_up\": unknown function \"same\"");
}

#[test]
fn warns_and_skips_unsupported_rules() {
    let rules = "\
domain: customers
blocking:
  - [tax_id]
rules:
  - name: fuzzy_name
    when:
      - similarity(legal_name) >= 0.85
      - equal(postcode)
    action: review
  - name: different_tax_id
    when: [different(tax_id), valid(tax_id)]
    action: never_merge
  - name: same_tax_id
    when: [equal(tax_id)]
survivorship:
  tax_id: [valid, most_recent]
";
    let project = load_ok(&with("rules/customers.yaml", rules));
    let warnings: Vec<String> = project.warnings.iter().map(ToString::to_string).collect();
    assert_has(
        &warnings,
        "rules/customers.yaml: blocking is not supported yet",
    );
    assert_has(
        &warnings,
        "rule \"fuzzy_name\" is skipped, not supported yet: action review, similarity",
    );
    assert_has(&warnings, "rule \"different_tax_id\" is skipped, not supported yet: action never_merge, different, valid");
    assert_has(
        &warnings,
        "survivorship of \"tax_id\": valid is not supported yet",
    );

    let set = project.rules_of("customers").unwrap();
    let names: Vec<&str> = set.rules.iter().map(|rule| rule.name.as_str()).collect();
    assert_eq!(names, ["same_tax_id"]);
    assert_eq!(set.survivorship["tax_id"], vec![Strategy::MostRecent]);
}

#[test]
fn rejects_bad_survivorship() {
    let rules = "\
domain: customers
survivorship:
  tax_id: [source(sap)]
  vat: [most_recent]
  email: [newest]
";
    let errors = load_errors(&with("rules/customers.yaml", rules));
    assert_has(
        &errors,
        "source \"sap\" does not exist in domain \"customers\"",
    );
    assert_has(&errors, "attribute \"vat\" does not exist");
    assert_has(&errors, "unknown survivorship strategy \"newest\"");
}

#[test]
fn rejects_rules_for_unknown_or_repeated_domains() {
    let errors = load_errors(&with("rules/other.yaml", "domain: suppliers\n"));
    assert_has(&errors, "rules/other.yaml: unknown domain \"suppliers\"");

    let errors = load_errors(&with("rules/again.yaml", "domain: customers\n"));
    assert_has(&errors, "domain \"customers\" has more than one rules file");
}

#[test]
fn collects_every_error() {
    let rules = "\
domain: customers
rules:
  - name: a
    when: [equal(nope)]
    confidence: -1
";
    let errors = load_errors(&with("rules/customers.yaml", rules));
    assert_eq!(errors.len(), 2, "{errors:#?}");
}

#[test]
fn ignores_files_that_are_not_yaml() {
    let mut files = valid_files();
    files.push(("sources/README.md", "not yaml: [\n"));
    let project = load_ok(&files);
    assert_eq!(project.sources.len(), 2);
}
