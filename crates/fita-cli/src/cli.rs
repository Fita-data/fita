//! Command line arguments.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

/// Master data management engine.
#[derive(Debug, Parser)]
#[command(
    name = "fita",
    version,
    about = "Master data management engine",
    after_help = "\
Examples:
  fita init my-project --demo
  fita validate --project my-project
  fita run customers --project my-project
  fita export customers --project my-project --format csv"
)]
pub struct Cli {
    /// Format of the messages printed by the command.
    #[arg(long, global = true, value_enum, default_value_t = Output::Text)]
    pub output: Output,

    #[command(subcommand)]
    pub command: Command,
}

/// Format of the messages printed by a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Output {
    /// Plain text for people.
    Text,
    /// One JSON document for scripts.
    Json,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Create a new project folder.
    #[command(after_help = "\
Examples:
  fita init my-project            empty project with a customers domain
  fita init my-project --demo     demo project with synthetic data")]
    Init(InitArgs),

    /// Check that a project is valid without running it.
    #[command(after_help = "\
Examples:
  fita validate --project my-project
  fita validate --project my-project --output json")]
    Validate(ValidateArgs),

    /// Read, normalize, match and build golden records for one domain.
    #[command(after_help = "\
Examples:
  fita run customers --project my-project
  fita run customers --project my-project --output json

Results are saved under <project>/.fita/.")]
    Run(RunArgs),

    /// Write the golden records of a domain as CSV or JSON.
    #[command(after_help = "\
Examples:
  fita export customers --project my-project --format csv
  fita export customers --project my-project --format json --out customers.json

Run the domain first with fita run. Without --out the data goes to stdout.")]
    Export(ExportArgs),
}

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Folder to create. It must not exist or be empty.
    pub dir: PathBuf,

    /// Fill the project with the demo: a customers domain and two CSV
    /// sources with synthetic data.
    #[arg(long)]
    pub demo: bool,
}

#[derive(Debug, Args)]
pub struct ValidateArgs {
    /// Project folder.
    #[arg(long)]
    pub project: PathBuf,
}

#[derive(Debug, Args)]
pub struct RunArgs {
    /// Domain to run, such as customers.
    pub domain: String,

    /// Project folder.
    #[arg(long)]
    pub project: PathBuf,
}

#[derive(Debug, Args)]
pub struct ExportArgs {
    /// Domain to export, such as customers.
    pub domain: String,

    /// Project folder.
    #[arg(long)]
    pub project: PathBuf,

    /// Data format.
    #[arg(long, value_enum)]
    pub format: ExportFormat,

    /// File to write. Without it the data goes to stdout.
    #[arg(long)]
    pub out: Option<PathBuf>,
}

/// Format of exported data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ExportFormat {
    /// One row per golden record, one column per attribute.
    Csv,
    /// Every golden record with members, links and provenance of each value.
    Json,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn arguments_are_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_the_quick_start_commands() {
        let cli = Cli::try_parse_from(["fita", "init", "my-project", "--demo"]).unwrap();
        assert!(matches!(
            cli.command,
            Command::Init(InitArgs { demo: true, .. })
        ));

        let cli = Cli::try_parse_from([
            "fita",
            "export",
            "customers",
            "--project",
            "p",
            "--format",
            "csv",
        ])
        .unwrap();
        let Command::Export(args) = cli.command else {
            panic!("expected export");
        };
        assert_eq!(args.format, ExportFormat::Csv);
        assert_eq!(args.out, None);
        assert_eq!(cli.output, Output::Text);
    }

    #[test]
    fn export_needs_a_format() {
        assert!(Cli::try_parse_from(["fita", "export", "customers", "--project", "p"]).is_err());
        assert!(Cli::try_parse_from([
            "fita",
            "export",
            "customers",
            "--project",
            "p",
            "--format",
            "xml"
        ])
        .is_err());
    }
}
