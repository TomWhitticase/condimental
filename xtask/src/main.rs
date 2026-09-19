//! Build script for the Rust <-> Godot round trip (ARCHITECTURE.md §7).
//!
//! `cargo xtask [--release]` runs `cargo build -p sim_core` and copies the
//! resulting cdylib into `godot_project/bin/`, where `sim_core.gdextension`
//! expects to find it. This is the whole point of a checked-in build
//! script: nobody should have to know or hand-copy the target path.
//!
//! It then stages the generated voxel models (UI_UX.md §2a) from
//! `assets/voxels/models/` into `godot_project/voxels/`, and the generated UI
//! icons (UI_UX.md §5) from `assets/voxels/icons/` into
//! `godot_project/voxels/icons/`, for exactly the same reason and with
//! exactly the same treatment: Godot can only load from inside `res://`,
//! the committed source of truth lives outside it, and neither copy should
//! ever be made by hand. Regenerating either is a separate, explicit step
//! (`cargo run -p voxgen --bin voxgen` / `cargo run -p voxgen --bin voxicons`, or
//! `scripts/build-voxels.sh` which does both plus previews) — xtask only
//! copies what is already there, so the common build stays fast.

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

    copy_voxel_models(&root);
    copy_voxel_icons(&root);

    if env::args().any(|a| a == "--no-import") {
        println!("xtask: --no-import given; run `{IMPORT_HINT}` (twice) before launching");
    } else {
        reimport_godot_assets(&root);
    }
}

/// The command a human has to run if xtask cannot run it for them.
const IMPORT_HINT: &str = "godot --headless --path godot_project --import";

/// Re-runs Godot's asset import over `godot_project/`, twice.
///
/// Staging the voxels above deletes `godot_project/voxels/` and rebuilds it,
/// which takes every `.import` sidecar with it — so straight after an xtask the
/// project has models Godot has never imported, and a windowed launch or a
/// `scripts/screenshot-scene.sh` render draws NO MESHES AT ALL (`floor_tile
/// mesh missing`). That cost real time to diagnose more than once, because the
/// failure looks like a broken scene rather than a missing build step.
///
/// It runs TWICE for the reason `scripts/verify-core-loop.sh` documents at
/// length: the `.vox` importer is an editor plugin, and it only registers on
/// the pass AFTER the first scan, so a cold cache imports zero models on pass
/// one and all of them on pass two.
///
/// Doing it here rather than in each script is the same argument the rest of
/// this file makes: xtask is what invalidated the sidecars, so xtask is what
/// should restore them, and nobody should have to know that. `--no-import`
/// opts out for a build that is not about to launch anything; a missing
/// `godot` is a printed reminder rather than a failure, since building the
/// library is useful on a machine with no engine installed (CI runs
/// `cargo test` alone).
fn reimport_godot_assets(root: &Path) {
    let godot_bin = env::var("GODOT_BIN").unwrap_or_else(|_| "godot".into());
    let project = root.join("godot_project");

    for pass in 1..=2 {
        let result = Command::new(&godot_bin)
            .current_dir(root)
            .args(["--headless", "--path"])
            .arg(&project)
            .arg("--import")
            .output();
        match result {
            Ok(output) if output.status.success() => {
                println!("xtask: godot --import pass {pass}/2 ok");
            }
            Ok(output) => {
                eprintln!(
                    "xtask: godot --import pass {pass}/2 failed ({}); \
                     run `{IMPORT_HINT}` (twice) by hand before launching",
                    output.status
                );
                return;
            }
            Err(e) => {
                println!(
                    "xtask: '{godot_bin}' not runnable ({e}); skipping asset import. \
                     Before launching the scene, run `{IMPORT_HINT}` TWICE \
                     (set GODOT_BIN to override), or the world draws no meshes."
                );
                return;
            }
        }
    }
}

/// Mirrors `assets/voxels/models/` into `godot_project/voxels/`.
///
/// Deletes stale entries first: a model removed from a generator must not
/// linger in the Godot project, or the scene would keep rendering an asset
/// the pipeline no longer produces.
fn copy_voxel_models(root: &Path) {
    let src_dir = root.join("assets").join("voxels").join("models");
    let dest_dir = root.join("godot_project").join("voxels");

    if !src_dir.is_dir() {
        eprintln!(
            "xtask: no {} — run `cargo run -p voxgen --bin voxgen` to generate the voxel models",
            src_dir.display()
        );
        return;
    }

    if dest_dir.exists() {
        fs::remove_dir_all(&dest_dir).expect("failed to clear godot_project/voxels");
    }
    fs::create_dir_all(&dest_dir).expect("failed to create godot_project/voxels");

    let mut copied = 0usize;
    let mut entries: Vec<_> = fs::read_dir(&src_dir)
        .expect("failed to read assets/voxels/models")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    entries.sort();

    for path in entries {
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().expect("directory entry must have a name");
        fs::copy(&path, dest_dir.join(name)).unwrap_or_else(|e| {
            panic!(
                "xtask: failed to copy {} into godot_project/voxels: {e}",
                path.display()
            )
        });
        copied += 1;
    }

    println!(
        "xtask: staged {copied} voxel asset files -> {}",
        dest_dir.display()
    );
}

/// Mirrors `assets/voxels/icons/` into `godot_project/voxels/icons/` — same
/// treatment and same reason as `copy_voxel_models` above: Godot can only
/// load from inside `res://`, and this is what `main.gd`'s `_icon_for`
/// (UI_UX.md §5) actually loads at runtime.
///
/// Unlike models, a missing `icons/` directory isn't a hard error: icon
/// generation needs `vengi-thumbnailer` installed (`cargo run -p voxgen
/// --bin voxicons`, or `scripts/build-voxels.sh`) the same way preview
/// rendering does, so a machine that has never had `vengi` installed and
/// hasn't pulled a commit with icons yet simply has no icons to stage —
/// `_icon_for`'s own documented placeholder fallback covers that case at
/// runtime, this just shouldn't fail the whole build over it.
fn copy_voxel_icons(root: &Path) {
    let src_dir = root.join("assets").join("voxels").join("icons");
    let dest_dir = root.join("godot_project").join("voxels").join("icons");

    if !src_dir.is_dir() {
        eprintln!(
            "xtask: no {} — run `cargo run -p voxgen --bin voxicons` to generate icons",
            src_dir.display()
        );
        return;
    }

    if dest_dir.exists() {
        fs::remove_dir_all(&dest_dir).expect("failed to clear godot_project/voxels/icons");
    }
    fs::create_dir_all(&dest_dir).expect("failed to create godot_project/voxels/icons");

    let mut copied = 0usize;
    let mut entries: Vec<_> = fs::read_dir(&src_dir)
        .expect("failed to read assets/voxels/icons")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    entries.sort();

    for path in entries {
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().expect("directory entry must have a name");
        fs::copy(&path, dest_dir.join(name)).unwrap_or_else(|e| {
            panic!(
                "xtask: failed to copy {} into godot_project/voxels/icons: {e}",
                path.display()
            )
        });
        copied += 1;
    }

    println!(
        "xtask: staged {copied} icon files -> {}",
        dest_dir.display()
    );
}
