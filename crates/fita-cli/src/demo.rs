//! Files of `examples/demo`, embedded so `fita init --demo` works from any
//! folder.

/// Every file of the demo project, as (relative path, content).
pub const FILES: &[(&str, &str)] = &[
    (
        "fita.yaml",
        include_str!("../../../examples/demo/fita.yaml"),
    ),
    (
        "domains/customers.yaml",
        include_str!("../../../examples/demo/domains/customers.yaml"),
    ),
    (
        "sources/billing.yaml",
        include_str!("../../../examples/demo/sources/billing.yaml"),
    ),
    (
        "sources/crm.yaml",
        include_str!("../../../examples/demo/sources/crm.yaml"),
    ),
    (
        "rules/customers.yaml",
        include_str!("../../../examples/demo/rules/customers.yaml"),
    ),
    (
        "data/billing.csv",
        include_str!("../../../examples/demo/data/billing.csv"),
    ),
    (
        "data/crm.csv",
        include_str!("../../../examples/demo/data/crm.csv"),
    ),
];

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::FILES;

    fn files_under(root: &Path, dir: &Path, found: &mut BTreeSet<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().is_some_and(|name| name == ".fita") {
                continue;
            }
            if path.is_dir() {
                files_under(root, &path, found);
            } else {
                let relative = path.strip_prefix(root).unwrap();
                found.insert(relative.to_string_lossy().replace('\\', "/"));
            }
        }
    }

    /// Catches a file added to examples/demo but not embedded here.
    #[test]
    fn embeds_every_file_of_examples_demo() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/demo");
        let mut on_disk = BTreeSet::new();
        files_under(&root, &root, &mut on_disk);
        let embedded: BTreeSet<String> = FILES.iter().map(|(path, _)| path.to_string()).collect();
        assert_eq!(embedded, on_disk);
    }
}
