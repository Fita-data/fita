//! One module per subcommand. Each `run` returns the exit code or an error
//! that `main` prints.

pub mod export;
pub mod init;
pub mod run;
pub mod validate;

use serde::Serialize;

/// Prints `value` as pretty JSON on stdout.
fn print_json<T: Serialize>(value: &T) -> anyhow::Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
