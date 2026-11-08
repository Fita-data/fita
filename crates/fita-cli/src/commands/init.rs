use std::fs;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::cli::{InitArgs, Output};
use crate::demo;

/// Written into every new project so run results stay out of git.
const GITIGNORE: &str = "# Data written by fita run\n.fita/\n";

const EMPTY_DOMAIN: &str = "\
name: customers
prefix: cus
attributes:
  - { name: tax_id, type: tax_id }
  - { name: legal_name, type: company_name }
  - { name: email, type: email, personal: true }
  - { name: phone, type: phone, personal: true }
";

#[derive(Serialize)]
struct Report<'a> {
    project: &'a Path,
    demo: bool,
    files: Vec<&'a str>,
}

pub fn run(args: &InitArgs, output: Output) -> Result<ExitCode> {
    ensure_empty(&args.dir)?;

    let settings;
    let mut files: Vec<(&str, &str)> = if args.demo {
        demo::FILES.to_vec()
    } else {
        settings = format!(
            "name: {}\n",
            serde_json::to_string(&project_name(&args.dir))?
        );
        vec![
            ("fita.yaml", settings.as_str()),
            ("domains/customers.yaml", EMPTY_DOMAIN),
        ]
    };
    files.push((".gitignore", GITIGNORE));

    for folder in ["domains", "sources", "rules"] {
        create_dir(&args.dir.join(folder))?;
    }
    for (path, content) in &files {
        let full = args.dir.join(path);
        if let Some(parent) = full.parent() {
            create_dir(parent)?;
        }
        fs::write(&full, content).with_context(|| format!("cannot write {}", full.display()))?;
    }

    match output {
        Output::Json => super::print_json(&Report {
            project: &args.dir,
            demo: args.demo,
            files: files.iter().map(|(path, _)| *path).collect(),
        })?,
        Output::Text => {
            let kind = if args.demo { "demo project" } else { "project" };
            println!("Created {kind} in {}", args.dir.display());
            println!("Next: fita validate --project {}", args.dir.display());
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn ensure_empty(dir: &Path) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    if !dir.is_dir() {
        bail!("{} exists and is not a folder", dir.display());
    }
    let mut entries =
        fs::read_dir(dir).with_context(|| format!("cannot read {}", dir.display()))?;
    if entries.next().is_some() {
        bail!("{} is not empty, choose a new folder", dir.display());
    }
    Ok(())
}

fn create_dir(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))
}

/// Project name taken from the folder name.
fn project_name(dir: &Path) -> String {
    dir.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "fita-project".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn creates_a_valid_empty_project() {
        let dir = TempDir::new().unwrap();
        let project = dir.path().join("acme: test");
        let args = InitArgs {
            dir: project.clone(),
            demo: false,
        };
        run(&args, Output::Json).unwrap();

        let loaded = fita_config::load(&project).unwrap();
        assert_eq!(loaded.name, "acme: test");
        assert!(loaded.domain("customers").is_some());
        assert!(loaded.sources.is_empty());
        assert!(project.join("sources").is_dir());
        assert_eq!(
            fs::read_to_string(project.join(".gitignore")).unwrap(),
            GITIGNORE
        );
    }

    #[test]
    fn creates_the_demo() {
        let dir = TempDir::new().unwrap();
        let project = dir.path().join("demo");
        let args = InitArgs {
            dir: project.clone(),
            demo: true,
        };
        run(&args, Output::Json).unwrap();
        let loaded = fita_config::load(&project).unwrap();
        assert_eq!(loaded.sources.len(), 2);
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    }

    #[test]
    fn refuses_folders_with_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("keep.txt"), "x").unwrap();
        let args = InitArgs {
            dir: dir.path().to_path_buf(),
            demo: true,
        };
        let error = run(&args, Output::Text).unwrap_err();
        assert!(error.to_string().contains("is not empty"), "{error}");
        assert!(!dir.path().join("fita.yaml").exists());
    }

    #[test]
    fn accepts_an_existing_empty_folder() {
        let dir = TempDir::new().unwrap();
        let args = InitArgs {
            dir: dir.path().to_path_buf(),
            demo: false,
        };
        run(&args, Output::Json).unwrap();
        assert!(dir.path().join("fita.yaml").is_file());
    }
}
