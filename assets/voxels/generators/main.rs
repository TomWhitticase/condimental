//! `cargo run -p voxgen --bin voxgen` — regenerates every voxel model in
//! `assets/voxels/models/` from the generators in this directory
//! (UI_UX.md §2a).
//!
//! Each asset is written twice over:
//!
//! * `models/<name>.vox` — the whole asset in one file. This is what gets
//!   rendered to an isometric PNG for review (`scripts/build-voxels.sh`) and
//!   what a human would open in `vengi-voxedit`.
//! * `models/<name>.<class>.vox` — the same asset split by material class
//!   (matte / metal / emissive), one file per class it actually uses.
//!
//! The split exists because a voxel mesh imported into Godot carries vertex
//! colours but a single material, and UI_UX.md §4 requires steel to be
//! *noticeably* shinier than concrete and status lamps to actually emit.
//! One mesh per class, each with its own `StandardMaterial3D`, is what makes
//! that true; `main.gd` reassembles them into one node per asset.
//!
//! `models/materials.json`, `models/lamp_states.json` and `models/index.json`
//! are written alongside so Godot can rebuild those materials, learn what a
//! lit and an idle status lamp look like, and discover which classes each
//! asset has, without any of those facts being duplicated in GDScript — or in
//! the asset browser, which reads the same three files (issue #81).

mod assets;
mod palette;
mod vox;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use palette::{MaterialClass, Palette};

fn models_dir() -> PathBuf {
    // generators/ -> assets/voxels/
    Path::new(env!("CARGO_MANIFEST_DIR")).join("models")
}

fn main() {
    let palette = Palette::load();
    let out = models_dir();
    std::fs::create_dir_all(&out).expect("failed to create assets/voxels/models");

    let mut index: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    let mut written = 0usize;

    for asset in assets::ASSETS {
        let model = (asset.build)(&palette);
        assert!(
            !model.is_empty(),
            "generator '{}' produced an empty model",
            asset.name
        );

        vox::write_vox(&out.join(format!("{}.vox", asset.name)), &model, &palette)
            .expect("failed to write combined .vox");
        written += 1;

        let mut classes = Vec::new();
        for class in MaterialClass::all() {
            let part = model.filter_class(&palette, class);
            let path = out.join(format!("{}.{}.vox", asset.name, class.name()));
            if part.is_empty() {
                // A class this asset doesn't use must not leave a stale file
                // behind from a previous generator revision.
                let _ = std::fs::remove_file(&path);
                continue;
            }
            vox::write_vox(&path, &part, &palette).expect("failed to write per-class .vox");
            written += 1;
            classes.push(class.name());
        }

        println!(
            "voxgen: {:<20} {:>6} voxels  [{}]",
            asset.name,
            model.voxels.len(),
            classes.join(", ")
        );
        index.insert(asset.name, classes);
    }

    std::fs::write(out.join("materials.json"), palette.materials_json())
        .expect("failed to write materials.json");
    // The two states a status lamp is drawn in, resolved against the
    // `emissive` class — read by Godot and by the asset browser alike, so
    // neither has to keep its own copy of what "idle" looks like (issue #81).
    std::fs::write(out.join("lamp_states.json"), palette.lamp_states_json())
        .expect("failed to write lamp_states.json");
    std::fs::write(
        out.join("index.json"),
        serde_json::to_string_pretty(&index).expect("index must serialize"),
    )
    .expect("failed to write index.json");

    println!(
        "voxgen: wrote {written} .vox files + materials.json + lamp_states.json + index.json to {}",
        out.display()
    );
}
