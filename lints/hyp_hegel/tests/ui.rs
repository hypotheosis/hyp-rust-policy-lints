use std::env::current_exe;
use std::fs::read_dir;
use std::path::{Path, PathBuf};

#[test]
fn ui() {
    let deps = deps_dir();
    let hegel = rlib(&deps, "hegel");

    dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), "ui")
        .rustc_flags([
            "--test".to_owned(),
            // `plain_test.rs` compiles under any edition, but the hegel
            // fixtures name `hegel::` in attribute and type position, which
            // needs the 2018-or-later extern prelude.
            "--edition=2024".to_owned(),
            // `dylint_testing` only recovers a fixture's `--extern`/`-L` flags
            // for *example* targets, so fixtures under `src_base` have to be
            // linked by hand. `-L dependency=` is what lets rustc find the
            // `hegel_macros` proc-macro crate that `hegel` re-exports from.
            "-L".to_owned(),
            format!("dependency={}", deps.display()),
            "--extern".to_owned(),
            format!("hegel={}", hegel.display()),
        ])
        .run();
}

/// The `target/debug/deps` directory holding this test binary and the
/// dev-dependencies built alongside it.
fn deps_dir() -> PathBuf {
    current_exe()
        .expect("could not determine test executable path")
        .parent()
        .expect("test executable has no parent directory")
        .to_path_buf()
}

/// The most recently built `lib<name>-<hash>.rlib` in `dir`.
///
/// Cargo leaves stale artifacts behind, so several hashes can coexist; the
/// newest is the one the current `Cargo.lock` produced.
fn rlib(dir: &Path, name: &str) -> PathBuf {
    let prefix = format!("lib{name}-");
    let mut candidates = read_dir(dir)
        .unwrap_or_else(|e| panic!("could not read `{}`: {e}", dir.display()))
        .filter_map(Result::ok)
        .filter(|entry| {
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            file_name.starts_with(&prefix) && file_name.ends_with(".rlib")
        })
        .map(|entry| {
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .expect("could not stat candidate rlib");
            (modified, entry.path())
        })
        .collect::<Vec<_>>();

    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| {
            panic!(
                "no `{prefix}*.rlib` in `{}`; is the `hegel_stub` dev-dependency built?",
                dir.display()
            )
        })
        .1
}
