//! A bad shell -- small cross platform module for writing glue code
//! This is based on https://github.com/rust-analyzer/rust-analyzer/blob/master/xtask/src/not_bash.rs
//! Feel free to copy-paste any relevant code from there or add custom one.

use anyhow::{bail, Context, Result};
use std::process::{Command, Stdio};

pub(crate) mod fs2 {
    use std::{ffi::OsStr, fs, path::Path};

    use anyhow::{Context, Result};

    pub(crate) fn read_files<'a>(
        dir_path: &'a Path,
        file_extensions: &'a [&'a str],
    ) -> Result<impl Iterator<Item = fs::DirEntry> + 'a> {
        Ok(read_dir(dir_path)?.filter_map(move |dir_entry| {
            let dir_entry = dir_entry.ok()?;
            if !dir_entry.file_type().ok()?.is_file() {
                return None;
            }
            file_extensions
                .iter()
                .find(|expected| file_extension(&dir_entry.file_name()) == Some(expected))
                .map(|_| dir_entry)
        }))
    }

    fn file_extension(name: &OsStr) -> Option<&str> {
        Path::new(name).extension().and_then(|it| it.to_str())
    }

    // std::fs has quite poor error messages...

    fn read_dir(path: impl AsRef<Path>) -> Result<fs::ReadDir> {
        let path = path.as_ref();
        fs::read_dir(path).with_context(|| format!("Failed to read dir {}", path.display()))
    }

    pub(crate) fn read_to_string(path: impl AsRef<Path>) -> Result<String> {
        let path = path.as_ref();
        fs::read_to_string(path).with_context(|| format!("Failed to read file {}", path.display()))
    }

    pub(crate) fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<()> {
        let path = path.as_ref();
        fs::write(path, contents)
            .with_context(|| format!("Failed to write file {}", path.display()))
    }

    pub(crate) fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> Result<u64> {
        let from = from.as_ref();
        let to = to.as_ref();
        fs::copy(from, to)
            .with_context(|| format!("Failed to copy {} to {}", from.display(), to.display()))
    }
}

macro_rules! _run {
    ($($expr:expr),*) => {
        run!($($expr),*; echo = true)
    };
    ($($expr:expr),* ; echo = $echo:expr) => {
        $crate::not_bash::run_process(format!($($expr),*), $echo)
    };
}
pub(crate) use _run as run;

#[doc(hidden)]
pub(crate) fn run_process(cmd: String, echo: bool) -> Result<String> {
    run_process_inner(&cmd, echo).with_context(|| format!("process `{}` failed", cmd))
}

fn run_process_inner(cmd: &str, echo: bool) -> Result<String> {
    let mut args = shelx(cmd);
    let binary = args.remove(0);

    if echo {
        println!("> {}", cmd)
    }

    let output = Command::new(binary)
        .args(args)
        .stdin(Stdio::null())
        .stderr(Stdio::inherit())
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;

    if echo {
        print!("{}", stdout)
    }

    if !output.status.success() {
        bail!("{}", output.status)
    }

    Ok(stdout.trim().to_string())
}

// Some fake shell lexing
fn shelx(cmd: &str) -> Vec<String> {
    cmd.split_whitespace().map(|it| it.to_string()).collect()
}
