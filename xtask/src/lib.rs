pub mod codegen;
mod not_bash;
pub mod pre_commit;

use std::{
    env,
    path::{Path, PathBuf},
};

fn project_root_dir() -> PathBuf {
    let manifest_dir =
        env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_owned());

    Path::new(&manifest_dir).parent().unwrap().to_owned()
}
