//! Reading project files and turning them into validated types.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use fita_core::{is_valid_prefix, Strategy};
use fita_normalize::AttributeType;
use serde::de::DeserializeOwned;

use crate::model::{
    Attribute, Connector, Domain, MatchRule, Project, RuleSet, Source, DEFAULT_CSV_SEPARATOR,
};
use crate::raw::{RawDomain, RawRule, RawRules, RawSettings, RawSource};
use crate::rules::{
    is_valid_name, parse_condition, parse_strategy, rejoin_split_calls, Condition, ParsedStrategy,
};
use crate::{Issue, LoadError};

const SETTINGS_FILE: &str = "fita.yaml";
const DOMAINS_DIR: &str = "domains";
const SOURCES_DIR: &str = "sources";
const RULES_DIR: &str = "rules";

/// Loads and validates the project in `root`.
///
/// Returns every error found, not only the first one. Valid files that use
/// features not supported yet load fine and leave a warning in
/// [`Project::warnings`].
pub fn load(root: &Path) -> Result<Project, LoadError> {
    if !root.join(SETTINGS_FILE).is_file() {
        return Err(LoadError::NotAProject(root.to_path_buf()));
    }
    let mut loader = Loader {
        root,
        errors: Vec::new(),
        warnings: Vec::new(),
    };

    let name = loader.load_settings();
    let domains = loader.load_domains();
    let sources = loader.load_sources(&domains);
    let rules = loader.load_rules(&domains, &sources);

    if !loader.errors.is_empty() {
        return Err(LoadError::Invalid(loader.errors));
    }
    Ok(Project {
        root: root.to_path_buf(),
        name: name.unwrap_or_default(),
        domains,
        sources,
        rules,
        warnings: loader.warnings,
    })
}

struct Loader<'a> {
    root: &'a Path,
    errors: Vec<Issue>,
    warnings: Vec<Issue>,
}

impl Loader<'_> {
    fn error(&mut self, path: &Path, message: impl Into<String>) {
        self.errors.push(Issue::new(path, message));
    }

    fn warning(&mut self, path: &Path, message: impl Into<String>) {
        self.warnings.push(Issue::new(path, message));
    }

    /// Reads and parses one YAML file. `path` is relative to the project.
    fn read_yaml<T: DeserializeOwned>(&mut self, path: &Path) -> Option<T> {
        let text = match fs::read_to_string(self.root.join(path)) {
            Ok(text) => text,
            Err(error) => {
                self.error(path, format!("cannot read file: {error}"));
                return None;
            }
        };
        match serde_yaml::from_str(&text) {
            Ok(value) => Some(value),
            Err(error) => {
                self.error(path, format!("invalid YAML: {error}"));
                None
            }
        }
    }

    /// Reads every `.yaml` or `.yml` file of a project folder, sorted by
    /// path. A missing folder counts as empty.
    fn read_folder<T: DeserializeOwned>(&mut self, folder: &str) -> Vec<(PathBuf, T)> {
        let dir = self.root.join(folder);
        if !dir.exists() {
            return Vec::new();
        }
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) => {
                self.error(Path::new(folder), format!("cannot read folder: {error}"));
                return Vec::new();
            }
        };
        let mut paths = Vec::new();
        for entry in entries {
            match entry {
                Ok(entry) if is_yaml(&entry.path()) => {
                    paths.push(Path::new(folder).join(entry.file_name()))
                }
                Ok(_) => {}
                Err(error) => self.error(Path::new(folder), format!("cannot read folder: {error}")),
            }
        }
        paths.sort();

        let mut files = Vec::new();
        for path in paths {
            if let Some(value) = self.read_yaml(&path) {
                files.push((path, value));
            }
        }
        files
    }

    fn load_settings(&mut self) -> Option<String> {
        let path = Path::new(SETTINGS_FILE);
        let settings: RawSettings = self.read_yaml(path)?;
        if settings.name.trim().is_empty() {
            self.error(path, "name must not be empty");
        }
        if settings.packs.is_some() {
            self.warning(path, "packs are not supported yet and are ignored");
        }
        if settings.working_store.is_some() {
            self.warning(path, "working_store is not supported yet and is ignored");
        }
        Some(settings.name)
    }

    fn load_domains(&mut self) -> Vec<Domain> {
        let files: Vec<(PathBuf, RawDomain)> = self.read_folder(DOMAINS_DIR);
        if files.is_empty() && self.errors.is_empty() {
            self.error(Path::new(DOMAINS_DIR), "the project has no domains");
        }
        let mut domains = Vec::new();
        let mut names = BTreeSet::new();
        let mut prefixes = BTreeSet::new();
        for (path, raw) in files {
            if !is_valid_name(&raw.name) {
                self.error(&path, format!("invalid domain name {:?}", raw.name));
            } else if !names.insert(raw.name.clone()) {
                self.error(
                    &path,
                    format!("domain {:?} is defined more than once", raw.name),
                );
            }
            if !is_valid_prefix(&raw.prefix) {
                self.error(
                    &path,
                    format!(
                        "invalid prefix {:?}, use lowercase letters and digits",
                        raw.prefix
                    ),
                );
            } else if !prefixes.insert(raw.prefix.clone()) {
                self.error(
                    &path,
                    format!("prefix {:?} is used by another domain", raw.prefix),
                );
            }
            let attributes = self.domain_attributes(&path, raw.attributes);
            domains.push(Domain {
                name: raw.name,
                prefix: raw.prefix,
                attributes,
            });
        }
        domains.sort_by(|a, b| a.name.cmp(&b.name));
        domains
    }

    fn domain_attributes(
        &mut self,
        path: &Path,
        raw: Vec<crate::raw::RawAttribute>,
    ) -> Vec<Attribute> {
        if raw.is_empty() {
            self.error(path, "a domain needs at least one attribute");
        }
        let mut attributes = Vec::new();
        let mut names = BTreeSet::new();
        for attribute in raw {
            if !is_valid_name(&attribute.name) {
                self.error(path, format!("invalid attribute name {:?}", attribute.name));
            } else if !names.insert(attribute.name.clone()) {
                self.error(
                    path,
                    format!("attribute {:?} is defined more than once", attribute.name),
                );
            }
            match attribute.attribute_type.parse::<AttributeType>() {
                Ok(attribute_type) => attributes.push(Attribute {
                    name: attribute.name,
                    attribute_type,
                    personal: attribute.personal,
                }),
                Err(error) => self.error(path, format!("attribute {:?}: {error}", attribute.name)),
            }
        }
        attributes
    }

    fn load_sources(&mut self, domains: &[Domain]) -> Vec<Source> {
        let files: Vec<(PathBuf, RawSource)> = self.read_folder(SOURCES_DIR);
        let mut sources = Vec::new();
        let mut names = BTreeSet::new();
        for (path, raw) in files {
            if !is_valid_name(&raw.name) {
                self.error(&path, format!("invalid source name {:?}", raw.name));
            } else if !names.insert(raw.name.clone()) {
                self.error(
                    &path,
                    format!("source {:?} is defined more than once", raw.name),
                );
            }
            if let Some(source) = self.source(&path, raw, domains) {
                sources.push(source);
            }
        }
        sources.sort_by(|a, b| a.name.cmp(&b.name));
        sources
    }

    fn source(&mut self, path: &Path, raw: RawSource, domains: &[Domain]) -> Option<Source> {
        if raw.schedule.is_some() {
            self.warning(path, "schedule is not supported yet and is ignored");
        }
        let domain = domains.iter().find(|domain| domain.name == raw.domain);
        if domain.is_none() {
            self.error(path, format!("unknown domain {:?}", raw.domain));
        }
        if raw.key.trim().is_empty() {
            self.error(path, "key must name the column with the record id");
        }
        if raw
            .modified_at
            .as_deref()
            .is_some_and(|c| c.trim().is_empty())
        {
            self.error(path, "modified_at must name a column or be left out");
        }
        if raw.mapping.is_empty() {
            self.error(path, "mapping must map at least one attribute to a column");
        }
        for (attribute, column) in &raw.mapping {
            if let Some(domain) = domain {
                if domain.attribute(attribute).is_none() {
                    self.error(
                        path,
                        format!(
                            "attribute {attribute:?} does not exist in domain {:?}",
                            domain.name
                        ),
                    );
                }
            }
            if column.trim().is_empty() {
                self.error(
                    path,
                    format!("attribute {attribute:?} is mapped to an empty column"),
                );
            }
        }
        let connector = self.connector(path, raw.connector)?;
        Some(Source {
            name: raw.name,
            domain: raw.domain,
            connector,
            key: raw.key,
            modified_at: raw.modified_at,
            mapping: raw.mapping,
        })
    }

    fn connector(&mut self, path: &Path, raw: crate::raw::RawConnector) -> Option<Connector> {
        if raw.kind != "csv" {
            self.error(
                path,
                format!(
                    "connector type {:?} is not supported yet, only csv",
                    raw.kind
                ),
            );
            return None;
        }
        let Some(file) = raw.path.filter(|file| !file.trim().is_empty()) else {
            self.error(path, "a csv connector needs a path");
            return None;
        };
        let separator = match raw.separator.as_deref() {
            None => DEFAULT_CSV_SEPARATOR,
            Some(text) => match text.as_bytes() {
                [byte] if byte.is_ascii() && *byte != b'"' && *byte != b'\n' => *byte,
                _ => {
                    self.error(
                        path,
                        format!("separator {text:?} must be a single ASCII character"),
                    );
                    return None;
                }
            },
        };
        Some(Connector::Csv {
            path: self.root.join(file),
            separator,
        })
    }

    fn load_rules(&mut self, domains: &[Domain], sources: &[Source]) -> Vec<RuleSet> {
        let files: Vec<(PathBuf, RawRules)> = self.read_folder(RULES_DIR);
        let mut sets = Vec::new();
        let mut seen = BTreeSet::new();
        for (path, raw) in files {
            let Some(domain) = domains.iter().find(|domain| domain.name == raw.domain) else {
                self.error(&path, format!("unknown domain {:?}", raw.domain));
                continue;
            };
            if !seen.insert(raw.domain.clone()) {
                self.error(
                    &path,
                    format!("domain {:?} has more than one rules file", raw.domain),
                );
                continue;
            }
            if raw.blocking.is_some() {
                self.warning(&path, "blocking is not supported yet and is ignored");
            }
            if raw.model.is_some() {
                self.warning(
                    &path,
                    "the probabilistic model is not supported yet and is ignored",
                );
            }
            let rules = self.match_rules(&path, domain, raw.rules);
            let survivorship = self.survivorship(&path, domain, sources, raw.survivorship);
            sets.push(RuleSet {
                domain: raw.domain,
                version: raw.version,
                rules,
                survivorship,
            });
        }
        sets.sort_by(|a, b| a.domain.cmp(&b.domain));
        sets
    }

    fn match_rules(&mut self, path: &Path, domain: &Domain, raw: Vec<RawRule>) -> Vec<MatchRule> {
        let mut rules = Vec::new();
        let mut names = BTreeSet::new();
        for rule in raw {
            if rule.name.trim().is_empty() {
                self.error(path, "a rule needs a name");
            } else if !names.insert(rule.name.clone()) {
                self.error(
                    path,
                    format!("rule {:?} is defined more than once", rule.name),
                );
            }
            if let Some(rule) = self.match_rule(path, domain, rule) {
                rules.push(rule);
            }
        }
        rules
    }

    /// Validates one rule. Returns it only if it is valid and supported.
    fn match_rule(&mut self, path: &Path, domain: &Domain, rule: RawRule) -> Option<MatchRule> {
        let name = &rule.name;
        let confidence = rule.confidence.unwrap_or(1.0);
        let mut valid = true;
        let mut unsupported = BTreeSet::new();

        if !(0.0..=1.0).contains(&confidence) {
            self.error(
                path,
                format!("rule {name:?}: confidence must be between 0 and 1"),
            );
            valid = false;
        }
        match rule.action.as_deref() {
            None | Some("merge") => {}
            Some(action @ ("review" | "never_merge")) => {
                unsupported.insert(format!("action {action}"));
            }
            Some(other) => {
                self.error(
                    path,
                    format!(
                        "rule {name:?}: unknown action {other:?}, use merge, review or never_merge"
                    ),
                );
                valid = false;
            }
        }
        if rule.when.is_empty() {
            self.error(
                path,
                format!("rule {name:?}: when needs at least one condition"),
            );
            valid = false;
        }

        let mut attributes = Vec::new();
        for entry in &rejoin_split_calls(rule.when) {
            match parse_condition(entry) {
                Condition::Equal(attribute) => {
                    if domain.attribute(&attribute).is_none() {
                        self.error(
                            path,
                            format!("rule {name:?}: attribute {attribute:?} does not exist in domain {:?}", domain.name),
                        );
                        valid = false;
                    }
                    attributes.push(attribute);
                }
                Condition::Unsupported(what) => {
                    unsupported.insert(what);
                }
                Condition::Invalid(message) => {
                    self.error(path, format!("rule {name:?}: {message}"));
                    valid = false;
                }
            }
        }

        if !valid {
            return None;
        }
        if !unsupported.is_empty() {
            let list: Vec<String> = unsupported.into_iter().collect();
            self.warning(
                path,
                format!(
                    "rule {name:?} is skipped, not supported yet: {}",
                    list.join(", ")
                ),
            );
            return None;
        }
        Some(MatchRule {
            name: rule.name,
            attributes,
            confidence,
        })
    }

    fn survivorship(
        &mut self,
        path: &Path,
        domain: &Domain,
        sources: &[Source],
        raw: BTreeMap<String, Vec<serde_yaml::Value>>,
    ) -> BTreeMap<String, Vec<Strategy>> {
        let mut result = BTreeMap::new();
        for (attribute, entries) in raw {
            if domain.attribute(&attribute).is_none() {
                self.error(
                    path,
                    format!(
                        "survivorship: attribute {attribute:?} does not exist in domain {:?}",
                        domain.name
                    ),
                );
                continue;
            }
            let mut strategies = Vec::new();
            for entry in rejoin_split_calls(entries) {
                let serde_yaml::Value::String(entry) = entry else {
                    self.error(path, format!("survivorship of {attribute:?}: a strategy must be a text such as most_recent"));
                    continue;
                };
                match parse_strategy(&entry) {
                    ParsedStrategy::Supported(strategy) => {
                        self.check_strategy_sources(path, &attribute, domain, sources, &strategy);
                        strategies.push(strategy);
                    }
                    ParsedStrategy::Unsupported(name) => self.warning(
                        path,
                        format!("survivorship of {attribute:?}: {name} is not supported yet and is skipped"),
                    ),
                    ParsedStrategy::Invalid(message) => {
                        self.error(path, format!("survivorship of {attribute:?}: {message}"))
                    }
                }
            }
            result.insert(attribute, strategies);
        }
        result
    }

    fn check_strategy_sources(
        &mut self,
        path: &Path,
        attribute: &str,
        domain: &Domain,
        sources: &[Source],
        strategy: &Strategy,
    ) {
        let Strategy::Source(names) = strategy else {
            return;
        };
        for name in names {
            let belongs = sources
                .iter()
                .any(|source| &source.name == name && source.domain == domain.name);
            if !belongs {
                self.error(
                    path,
                    format!(
                        "survivorship of {attribute:?}: source {name:?} does not exist in domain {:?}",
                        domain.name
                    ),
                );
            }
        }
    }
}

fn is_yaml(path: &Path) -> bool {
    path.is_file()
        && matches!(
            path.extension().and_then(|ext| ext.to_str()),
            Some("yaml" | "yml")
        )
}

#[cfg(test)]
mod tests;
