//! Build script for the Rust <-> Godot round trip (ARCHITECTURE.md §7).
//!
//! `cargo xtask [--release]` runs `cargo build -p sim_core` and copies the
//! resulting cdylib into `godot_project/bin/`, where `sim_core.gdextension`
//! expects to find it. This is the whole point of a checked-in build
//! script: nobody should have to know or hand-copy the target path.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    // xtask/src/main.rs -> xtask/ -> workspace root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask crate must live one level below the workspace root")
        .to_path_buf()
}

/// (dylib filename in target/, filename Godot expects in godot_project/bin/)
fn lib_filenames() -> (String, String) {
    if cfg!(target_os = "windows") {
        ("sim_core.dll".into(), "sim_core.dll".into())
    } else if cfg!(target_os = "macos") {
        ("libsim_core.dylib".into(), "libsim_core.dylib".into())
    } else {
        ("libsim_core.so".into(), "libsim_core.so".into())
    }
}

fn main() {
    let release = env::args().any(|a| a == "--release");
    let root = workspace_root();

    let mut cmd = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
    cmd.current_dir(&root).args(["build", "-p", "sim_core"]);
    if release {
        cmd.arg("--release");
    }

    let status = cmd.status().expect("failed to spawn `cargo build`");
    if !status.success() {
        eprintln!("xtask: `cargo build -p sim_core` failed");
        std::process::exit(1);
    }

    let profile_dir = if release { "release" } else { "debug" };
    let (built_name, dest_name) = lib_filenames();

    let src = root.join("target").join(profile_dir).join(&built_name);
    let dest_dir = root.join("godot_project").join("bin");
    let dest = dest_dir.join(&dest_name);

    fs::create_dir_all(&dest_dir).expect("failed to create godot_project/bin");
    fs::copy(&src, &dest).unwrap_or_else(|e| {
        panic!(
            "xtask: failed to copy {} -> {}: {e}",
            src.display(),
            dest.display()
        )
    });

    println!("xtask: copied {} -> {}", src.display(), dest.display());
}
