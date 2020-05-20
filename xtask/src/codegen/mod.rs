pub mod github_workflows;

use crate::not_bash::fs2;
use anyhow::{bail, Result};
use std::path::Path;

#[derive(Copy, Clone)]
pub enum Mode {
    Verify,
    Overwrite,
}

fn ensure_codegen_freshness(expected: &str, generated_file_path: &Path, mode: Mode) -> Result<()> {
    match mode {
        Mode::Verify => {
            let actual = fs2::read_to_string(generated_file_path)?;
            if actual != expected {
                bail!(
                    "{} is not fresh, please run `cargo xtask codegen` or `cargo xtask install-pre-commit-hook [--force]`",
                    generated_file_path.display()
                );
            }
        }
        Mode::Overwrite => fs2::write(generated_file_path, expected)?,
    }
    Ok(())
}
