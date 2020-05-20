//! See the docs for `Args`

use anyhow::Result;
use std::env;
use xtask::{codegen, pre_commit};

const HELP: &str = "\
xtask
This binary defines auxiliary commands, which are not expressible with just `cargo`. Notably, it provides
`cargo xtask codegen` for code generation. This is a drop-in replacement for nasty enormous bash scripts.
This program is a dev cli, it is intended to be used only by the developers and not for production distribution.
See https://github.com/matklad/cargo-xtask/ for more info.

USAGE:
    xtask <SUBCOMMAND>

SUBCOMMANDS:
    codegen                       Generate github workflow file based of `xtask/src/codegen/github_workflows/*.yml` files.
                                  This expands yaml anchors along with removing any tracks of it. This is needed because GitHub workflows
                                  doesnt support yaml anchors. https://github.community/t5/GitHub-Actions/Support-for-YAML-anchors/td-p/30336
                                  This also replaces `<stable>` in yaml config file with the version specified in `rust-toolchain` file
                                  (this file is also used by rustup to override the default toolchain channel)

    install-pre-commit-hook       Install git pre-commit hook that formats code with `cargo fmt (use --force to overwrite)";

fn main() -> Result<()> {
    if let Some(true) = env::args().next().map(|it| it.contains("pre-commit")) {
        return pre_commit::run_hook();
    }

    let mut args = pico_args::Arguments::from_env();

    match args.subcommand()?.unwrap_or_default().as_str() {
        "codegen" => {
            codegen::github_workflows::generate(codegen::Mode::Overwrite)?;
            eprintln!("Codegen has finished, have a nice day!");
        }
        "install-pre-commit-hook" => {
            let force = args.contains("--force");
            args.finish()?;
            pre_commit::install_hook(force)?;
            eprintln!("Git pre-commit hook is successfully installed");
        }
        _ => eprintln!("{}", HELP),
    }
    Ok(())
}
