//! `cargo run -p voxgen --bin voxicons` — renders a small, UI-appropriate
//! icon PNG for every combined voxel model in `assets/voxels/models/`,
//! written to `assets/voxels/icons/` (UI_UX.md §5: "icons are rendered from
//! the real assets ... not separately-drawn 2D icon art"). It also renders
//! `APP_ICON_MODEL` a second time, larger, as `app_icon.png`: the 512px
//! window/dock icon `project.godot`'s `config/icon` points at.
//!
//! **Why this isn't just `assets/voxels/previews/`, reused as-is (a real
//! open question this binary resolves by checking, not assuming — see
//! UI_UX.md §2b):** a review preview and a UI icon want different things
//! from the same render step. A preview is meant to be looked at large,
//! with padding, and doesn't care about its background; a UI icon needs a
//! transparent background (to sit over UI chrome) and a tight, centered
//! crop at a small, consistent size. Checking the actual PNGs
//! `scripts/build-voxels.sh` already produces confirms they're the former:
//! `vengi-thumbnailer`'s `--image` mode always fills the whole frame with
//! an opaque solid colour — confirmed by reading its own `--jsonconfig` cvar
//! dump (there is no background/alpha/clear-colour cvar at all) and by
//! rendering the same model through `--image --isometric` AND the default
//! camera-based path and getting an identically opaque result both times.
//! So this is still "extend the existing offline pipeline" (UI_UX.md §2a
//! option (a)) in spirit, just not a bare reuse of the existing preview
//! files: it calls the same `vengi-thumbnailer` binary, then does the
//! transparency/crop/resize work `vengi-thumbnailer` itself has no flag for.
//!
//! Concretely, per model:
//!
//!   1. Render at `RENDER_SIZE` (larger than the final icon) with the same
//!      `--image --isometric` incantation `scripts/build-voxels.sh` uses.
//!   2. Flood-fill the background out to transparent, starting from the
//!      image's border pixels (which are always background, never model) —
//!      not a flat colour-key over the whole image, so a model that ever
//!      uses a near-black colour in its *interior* (none do today; see
//!      `palette.ron`) can't be falsely keyed out just for being dark.
//!   3. Crop to the remaining opaque content's bounding box, downscale onto
//!      a square `ICON_SIZE` (or `APP_ICON_SIZE`) canvas centered with a small margin. Rendering
//!      larger than the final size and downscaling is what keeps isometric
//!      cube edges from looking jagged at icon size.
//!
//! Like preview rendering, this needs `vengi-thumbnailer` installed — see
//! `scripts/build-voxels.sh`'s header for how to get it. Unlike the
//! gitignored `previews/`, `icons/` IS committed: `godot_project/main.gd`
//! loads these PNGs at runtime (via `res://voxels/icons/`, staged by
//! `cargo xtask` the same way `models/` is), so a clone of this repo must
//! have them without needing `vengi` installed just to run the game —
//! exactly the same treatment `models/` already gets relative to `voxgen`.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use image::{Rgba, RgbaImage};

/// Rendered larger than the final icon so downscaling can anti-alias the
/// isometric cube edges instead of leaving them jagged.
const RENDER_SIZE: u32 = 256;
/// Final icon size — a UI hover/hotbar icon slot (UI_UX.md §5), not a
/// full-scene review render.
const ICON_SIZE: u32 = 64;
/// A small breathing margin around the model's content, as a fraction of
/// the output size, so the icon doesn't touch its own edges.
const CROP_MARGIN_FRAC: f32 = 0.06;
/// The model the desktop app icon (`config/icon` in `project.godot`) is
/// rendered from: the ketchup bottle, the game's first finished product.
const APP_ICON_MODEL: &str = "item_bottle";
/// The window/dock icon is shown far larger than a UI slot, so it gets its
/// own bigger render rather than an upscale of the 64px icon.
const APP_ICON_RENDER_SIZE: u32 = 1024;
const APP_ICON_SIZE: u32 = 512;

fn voxels_root() -> PathBuf {
    // generators/ -> assets/voxels/
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Mirrors `scripts/build-voxels.sh`'s own thumbnailer lookup exactly, so
/// `VENGI_THUMBNAILER` (which the script already exports before calling
/// this binary) and the same fallback install path both work whether this
/// binary is run standalone or from that script.
fn find_thumbnailer() -> Option<PathBuf> {
    if let Ok(p) = env::var("VENGI_THUMBNAILER")
        && !p.is_empty()
    {
        return Some(PathBuf::from(p));
    }
    if let Some(p) = which_on_path("vengi-thumbnailer") {
        return Some(p);
    }
    let home = env::var("HOME").ok()?;
    let default =
        PathBuf::from(home).join(".local/vengi/thumbnailer.app/Contents/MacOS/vengi-thumbnailer");
    if default.is_file() {
        return Some(default);
    }
    None
}

fn which_on_path(bin: &str) -> Option<PathBuf> {
    let path = env::var("PATH").ok()?;
    for dir in env::split_paths(&path) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// `name_fN` with N >= 1 is the same asset one animation frame on (the belt
/// family, `assets.rs`'s conveyor section). An icon is a catalogue picture
/// of a kind, so only frame 0 gets one; rendering the rest would be three
/// near-identical PNGs per shape that nothing ever loads.
fn is_later_animation_frame(stem: &str) -> bool {
    stem.rsplit_once("_f")
        .and_then(|(_, n)| n.parse::<u32>().ok())
        .is_some_and(|n| n >= 1)
}

/// A `*_motion_fN` overlay (`assets.rs`'s "Machine motion" section) is half a
/// machine, not a machine: a handful of flywheel spokes or a ribbon, drawn on
/// top of a body that carries the rest. Frame 0 is skipped along with the
/// others — the rule above keeps one icon per animated thing because that one
/// is a picture of the thing, and a spider of four spokes floating on nothing
/// is not a picture of the generic machine.
fn is_motion_overlay(stem: &str) -> bool {
    stem.contains("_motion_f")
}

fn main() {
    let root = voxels_root();
    let models_dir = root.join("models");
    let icons_dir = root.join("icons");

    let Some(thumbnailer) = find_thumbnailer() else {
        eprintln!("warning: vengi-thumbnailer not found — icon generation SKIPPED.");
        eprintln!("         See scripts/build-voxels.sh's header for how to install it.");
        std::process::exit(0);
    };

    std::fs::create_dir_all(&icons_dir).expect("failed to create assets/voxels/icons");

    let mut entries: Vec<PathBuf> = std::fs::read_dir(&models_dir)
        .expect(
            "failed to read assets/voxels/models — run `cargo run -p voxgen --bin voxgen` first",
        )
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("vox"))
        .filter(|p| {
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            // Only the combined model is worth an icon — same rule
            // scripts/build-voxels.sh's preview loop already applies to the
            // per-material-class split files (name.matte.vox etc.).
            // "Has a dot in it" rather than a list of known class suffixes:
            // asset names are plain identifiers, and the hand-listed
            // version (".matte"/".metal"/".emissive") silently started
            // emitting a `pipe_fluid.liquid.png` the moment a fourth
            // material class was added in milestone 6 — exactly the drift a
            // literal list invites.
            !stem.contains('.') && !is_later_animation_frame(stem) && !is_motion_overlay(stem)
        })
        .collect();
    entries.sort();

    let tmp_dir = env::temp_dir();
    let mut failures = 0usize;
    let mut written = 0usize;

    for model_path in &entries {
        let name = model_path.file_stem().unwrap().to_str().unwrap();
        let out_path = icons_dir.join(format!("{name}.png"));
        match render_icon(
            &thumbnailer,
            model_path,
            &tmp_dir,
            RENDER_SIZE,
            ICON_SIZE,
            &out_path,
        ) {
            Ok(()) => {
                println!("  {}", out_path.display());
                written += 1;
            }
            Err(e) => {
                eprintln!("  FAIL {name} ({e})");
                failures += 1;
            }
        }
    }

    let app_icon_model = models_dir.join(format!("{APP_ICON_MODEL}.vox"));
    let app_icon_path = icons_dir.join("app_icon.png");
    let app_icon = render_icon(
        &thumbnailer,
        &app_icon_model,
        &tmp_dir,
        APP_ICON_RENDER_SIZE,
        APP_ICON_SIZE,
        &app_icon_path,
    );
    match app_icon {
        Ok(()) => {
            println!("  {}", app_icon_path.display());
            written += 1;
        }
        Err(e) => {
            eprintln!("  FAIL app_icon ({e})");
            failures += 1;
        }
    }

    if failures > 0 {
        eprintln!("voxicons: {failures} icon(s) failed");
        std::process::exit(1);
    }
    println!("voxicons: wrote {written} icons to {}", icons_dir.display());
}

/// Renders `model_path` at `render_size` and writes it to `out_path` as a
/// `size` icon — the three steps of the module doc for one model.
fn render_icon(
    thumbnailer: &Path,
    model_path: &Path,
    tmp_dir: &Path,
    render_size: u32,
    size: u32,
    out_path: &Path,
) -> Result<(), String> {
    let name = model_path.file_stem().unwrap().to_str().unwrap();
    let raw_path = tmp_dir.join(format!("voxicons_{name}_{render_size}_raw.png"));

    let status = Command::new(thumbnailer)
        .arg("-i")
        .arg(model_path)
        .arg("-o")
        .arg(&raw_path)
        .args(["-s", &render_size.to_string()])
        .args(["--image", "--isometric"])
        .status();

    if !matches!(status, Ok(s) if s.success()) {
        return Err("vengi-thumbnailer".to_string());
    }

    let raw = image::open(&raw_path)
        .map_err(|e| format!("reading render: {e}"))?
        .to_rgba8();
    let _ = std::fs::remove_file(&raw_path);

    make_icon(&raw, size)
        .save(out_path)
        .map_err(|e| format!("writing icon: {e}"))
}

/// Keys out the solid background `vengi-thumbnailer` fills the frame with,
/// crops to the remaining content, and returns a square `size` canvas
/// with the model centered on a transparent background.
fn make_icon(raw: &RgbaImage, size: u32) -> RgbaImage {
    let (w, h) = raw.dimensions();
    let bg = raw.get_pixel(0, 0).0; // the corner is always background
    let mut keyed = raw.clone();
    flood_key_background(&mut keyed, bg);

    let (min_x, min_y, max_x, max_y) = content_bbox(&keyed).unwrap_or((0, 0, w - 1, h - 1));
    let content_w = max_x - min_x + 1;
    let content_h = max_y - min_y + 1;
    let cropped = image::imageops::crop_imm(&keyed, min_x, min_y, content_w, content_h).to_image();

    // Fit inside `size` minus a small margin, preserving aspect ratio.
    let margin = ((size as f32 * CROP_MARGIN_FRAC).round() as u32) * 2;
    let target = size.saturating_sub(margin).max(1);
    let scale =
        (target as f32 / content_w.max(1) as f32).min(target as f32 / content_h.max(1) as f32);
    let new_w = ((content_w as f32) * scale).round().max(1.0) as u32;
    let new_h = ((content_h as f32) * scale).round().max(1.0) as u32;
    let resized = image::imageops::resize(
        &cropped,
        new_w,
        new_h,
        image::imageops::FilterType::Lanczos3,
    );

    let mut canvas = RgbaImage::from_pixel(size, size, Rgba([0, 0, 0, 0]));
    let off_x = (size - new_w) / 2;
    let off_y = (size - new_h) / 2;
    image::imageops::overlay(&mut canvas, &resized, off_x as i64, off_y as i64);
    canvas
}

/// BFS flood fill from every border pixel matching `bg` (within a small
/// tolerance), setting alpha to 0. Only pixels *connected* to the border
/// are keyed, so an interior voxel colour that happens to be dark (none
/// exist in `palette.ron` today, but nothing guarantees that forever)
/// can't be mistaken for background just because it's dark.
fn flood_key_background(img: &mut RgbaImage, bg: [u8; 4]) {
    let (w, h) = img.dimensions();
    let matches_bg = |p: &Rgba<u8>| {
        let d = |a: u8, b: u8| (a as i32 - b as i32).unsigned_abs();
        d(p.0[0], bg[0]) <= 4 && d(p.0[1], bg[1]) <= 4 && d(p.0[2], bg[2]) <= 4
    };

    let mut visited = vec![false; (w * h) as usize];
    let mut stack: Vec<(u32, u32)> = Vec::new();
    for x in 0..w {
        stack.push((x, 0));
        stack.push((x, h - 1));
    }
    for y in 0..h {
        stack.push((0, y));
        stack.push((w - 1, y));
    }

    while let Some((x, y)) = stack.pop() {
        let idx = (y * w + x) as usize;
        if visited[idx] {
            continue;
        }
        let pixel = *img.get_pixel(x, y);
        if !matches_bg(&pixel) {
            continue;
        }
        visited[idx] = true;
        img.put_pixel(x, y, Rgba([pixel.0[0], pixel.0[1], pixel.0[2], 0]));

        if x > 0 {
            stack.push((x - 1, y));
        }
        if x + 1 < w {
            stack.push((x + 1, y));
        }
        if y > 0 {
            stack.push((x, y - 1));
        }
        if y + 1 < h {
            stack.push((x, y + 1));
        }
    }
}

fn content_bbox(img: &RgbaImage) -> Option<(u32, u32, u32, u32)> {
    let (w, h) = img.dimensions();
    let mut min_x = w;
    let mut min_y = h;
    let mut max_x: i64 = -1;
    let mut max_y: i64 = -1;
    for y in 0..h {
        for x in 0..w {
            if img.get_pixel(x, y).0[3] > 0 {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x as i64);
                max_y = max_y.max(y as i64);
            }
        }
    }
    if max_x < 0 {
        return None;
    }
    Some((min_x, min_y, max_x as u32, max_y as u32))
}
