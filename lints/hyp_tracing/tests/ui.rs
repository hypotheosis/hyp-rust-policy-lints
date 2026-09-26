use std::env::current_exe;
use std::fs::read_dir;
use std::path::{Path, PathBuf};

/// Default config, ordinary (non-test) compilation.
#[test]
fn ui() {
    run("ui", Build::Library, None);
}

/// How a fixture directory is compiled.
#[derive(Clone, Copy)]
enum Build {
    /// `--crate-type=lib`: the fixtures need no `fn main`, which would itself
    /// be an uninstrumented function and put noise in every `.stderr`.
    Library,
}

fn run(src_base: &str, build: Build, dylint_toml: Option<&str>) {
    let deps = deps_dir();
    let tracing = rlib(&deps, "tracing");

    let mut flags = vec![
        // `#[tracing::instrument]` in attribute position needs the 2018-or-later
        // extern prelude.
        "--edition=2024".to_owned(),
        // `dylint_testing` only recovers a fixture's `--extern`/`-L` flags for
        // *example* targets, so fixtures under `src_base` are linked by hand.
        // `-L dependency=` is what lets rustc find `tracing_attributes` and
        // `tracing_core`, which `tracing` depends on.
        "-L".to_owned(),
        format!("dependency={}", deps.display()),
        "--extern".to_owned(),
        format!("tracing={}", tracing.display()),
    ];
    match build {
        Build::Library => flags.push("--crate-type=lib".to_owned()),
    }

    // Absolute, so compiletest's `$DIR` substitution matches only the fixture
    // directory's full path, not every "ui" inside words like "builds".
    let src_base = Path::new(env!("CARGO_MANIFEST_DIR")).join(src_base);
    let mut test = dylint_testing::ui::Test::src_base(env!("CARGO_PKG_NAME"), &src_base);
    test.rustc_flags(flags);
    if let Some(dylint_toml) = dylint_toml {
        test.dylint_toml(dylint_toml);
    }
    test.run();
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
/// newest is the one the current `Cargo.lock` produced. A copy of
/// `hyp_hegel/tests/ui.rs`'s helper: each lint crate is its own test binary
/// and there is no shared crate to put it in.
///
/// For `tracing` this relies on feature unification: `dylint_testing` pulls in
/// `tracing` via rustfix with only `std`, and hyp_hegel carries a `tracing`
/// dev-dependency solely so every workspace build asks for the same features
/// (incl. `attributes`) and produces one rlib. If a build without `attributes`
/// is ever newest, every fixture fails with "cannot find attribute
/// `instrument` in `tracing`"; `cargo clean -p tracing` clears the stale rlib.
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
                "no `{prefix}*.rlib` in `{}`; is the `tracing` dev-dependency built?",
                dir.display()
            )
        })
        .1
}
