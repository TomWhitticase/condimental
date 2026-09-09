//! A minimal MagicaVoxel `.vox` writer, plus the composable voxel
//! primitives every asset generator is built from (UI_UX.md §2a point 1).
//!
//! **Why a hand-written encoder rather than a crate** (UI_UX.md §9's open
//! question, resolved here): the three candidates the doc named were all
//! checked against what this pipeline actually needs.
//!
//! - `vox-format` 0.1.0 — still parser-only, exactly as the doc suspected.
//! - `vox_writer` 0.1.9 — does write SIZE/XYZI/RGBA, but emits no scene
//!   graph (`nTRN`/`nGRP`/`nSHP`) chunks. The pinned Godot importer
//!   (godot_project/addons/MagicaVoxel_Importer_with_Extensions) *starts*
//!   from scene-graph node 0 and cannot import a file without one, so its
//!   output is unusable here.
//! - `create-vox` — same shape of gap, and less maintained.
//!
//! The format itself is a handful of RIFF-style chunks (see UI_UX.md §10's
//! spec links), so encoding it directly is ~150 lines, has no dependency
//! risk, and — the actual deciding factor — lets us emit the scene graph and
//! `MATL` material chunks the importer and `vengi-voxconvert` both want.
//! Revisit only if a crate starts writing scene graphs.
//!
//! Coordinate convention: `x` is the world's +X (the direction the
//! production chain runs), `y` is depth, `z` is up. Godot's importer maps
//! vox `(x, y, z)` to world `(x, z, -y)`, so vox `z` is the vertical axis
//! throughout this module.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

use crate::palette::{MaterialClass, Palette};

/// One asset's voxel volume. `UI_UX.md §2` fixes this at 32×32×32 for a
/// one-tile asset — `TILE_VOXELS` below is that shared unit, and nothing in
/// the pipeline should quietly use a different one.
pub const TILE_VOXELS: u32 = 32;

#[derive(Clone)]
pub struct VoxModel {
    pub size: [u32; 3],
    /// (x, y, z) -> palette index (1-based, MagicaVoxel convention).
    /// `BTreeMap` rather than a hash map so the emitted `XYZI` voxel order
    /// is deterministic — a regenerated model must be byte-identical or the
    /// "models/ is a build artifact" rule (UI_UX.md §2a point 2) produces
    /// noisy diffs for no reason.
    pub voxels: BTreeMap<(u32, u32, u32), u8>,
}

impl VoxModel {
    pub fn new(size: [u32; 3]) -> Self {
        Self { size, voxels: BTreeMap::new() }
    }

    /// A standard one-tile asset volume (UI_UX.md §2's 32×32×32 scale).
    pub fn tile() -> Self {
        Self::new([TILE_VOXELS, TILE_VOXELS, TILE_VOXELS])
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, color: u8) {
        if x < 0 || y < 0 || z < 0 {
            return;
        }
        let (x, y, z) = (x as u32, y as u32, z as u32);
        if x >= self.size[0] || y >= self.size[1] || z >= self.size[2] {
            return;
        }
        self.voxels.insert((x, y, z), color);
    }

    /// Inclusive-bounds solid cuboid — the workhorse primitive.
    pub fn cuboid(
        &mut self,
        x: (i32, i32),
        y: (i32, i32),
        z: (i32, i32),
        color: u8,
    ) -> &mut Self {
        for zz in z.0..=z.1 {
            for yy in y.0..=y.1 {
                for xx in x.0..=x.1 {
                    self.set(xx, yy, zz, color);
                }
            }
        }
        self
    }

    /// Solid cylinder whose axis runs along +Z (an upright tank/vat).
    pub fn cylinder_z(
        &mut self,
        center: (i32, i32),
        radius: f32,
        z: (i32, i32),
        color: u8,
    ) -> &mut Self {
        self.cylinder_z_inner(center, radius, z, Some(color))
    }

    /// [`Self::cylinder_z`] in reverse — the upright counterpart of
    /// [`Self::carve_cylinder_x`], added for the mixer's open-topped
    /// blending vessel (the "Machines improvement" milestone): the agitator
    /// inside it is only visible if the drum is genuinely hollow, and a bore
    /// filled with an "air" colour would still mesh as a solid plug (see
    /// [`Self::erase`]).
    pub fn carve_cylinder_z(&mut self, center: (i32, i32), radius: f32, z: (i32, i32)) -> &mut Self {
        self.cylinder_z_inner(center, radius, z, None)
    }

    fn cylinder_z_inner(
        &mut self,
        center: (i32, i32),
        radius: f32,
        z: (i32, i32),
        color: Option<u8>,
    ) -> &mut Self {
        let r2 = radius * radius;
        let span = radius.ceil() as i32;
        for zz in z.0..=z.1 {
            for dy in -span..=span {
                for dx in -span..=span {
                    if (dx * dx + dy * dy) as f32 <= r2 {
                        match color {
                            Some(c) => self.set(center.0 + dx, center.1 + dy, zz, c),
                            None => self.erase(center.0 + dx, center.1 + dy, zz),
                        }
                    }
                }
            }
        }
        self
    }

    /// Removes a voxel, if one is there. The counterpart to [`Self::set`],
    /// and the reason the `carve_*` primitives below exist at all: the pipe
    /// (milestone 6) is the first asset that has to be *hollow* — a solid
    /// cylinder with a bore taken back out of it — and with a sight opening
    /// cut through its top wall so the fluid inside is actually visible
    /// (UI_UX.md §4). Filling the bore with an "air-coloured" voxel instead
    /// would be a lie the mesher can't see through: a face is only culled
    /// when its neighbour is *absent*, so an opening has to really be empty.
    pub fn erase(&mut self, x: i32, y: i32, z: i32) {
        if x < 0 || y < 0 || z < 0 {
            return;
        }
        self.voxels.remove(&(x as u32, y as u32, z as u32));
    }

    /// Is there a voxel at `(x, y, z)`? Reads the same map [`Self::set`]
    /// writes, so a test can assert an *absence* — which is the only way to
    /// check that a carved opening (a machine's inlet mouth, a pipe's bore)
    /// is genuinely open rather than filled with a dark colour that merely
    /// looks like a hole. See `assets.rs`'s connection tests.
    pub fn filled(&self, x: i32, y: i32, z: i32) -> bool {
        if x < 0 || y < 0 || z < 0 {
            return false;
        }
        self.voxels.contains_key(&(x as u32, y as u32, z as u32))
    }

    /// Inclusive-bounds cuboid of empty space — [`Self::cuboid`] in reverse.
    pub fn carve_cuboid(&mut self, x: (i32, i32), y: (i32, i32), z: (i32, i32)) -> &mut Self {
        for zz in z.0..=z.1 {
            for yy in y.0..=y.1 {
                for xx in x.0..=x.1 {
                    self.erase(xx, yy, zz);
                }
            }
        }
        self
    }

    /// Solid cylinder whose axis runs along +X — a pipe run laid along the
    /// world's +X axis, which is the direction both the belt chain and
    /// milestone 6's pipe run travel in (see `world.rs`).
    pub fn cylinder_x(
        &mut self,
        center: (i32, i32),
        radius: f32,
        x: (i32, i32),
        color: u8,
    ) -> &mut Self {
        self.cylinder_x_inner(center, radius, x, Some(color))
    }

    /// The bore: [`Self::cylinder_x`] in reverse, leaving empty space.
    /// (Kept as the pair of `cylinder_x` even while the pipes carve with
    /// the fractional-centre twin below.)
    #[allow(dead_code)]
    pub fn carve_cylinder_x(
        &mut self,
        center: (i32, i32),
        radius: f32,
        x: (i32, i32),
    ) -> &mut Self {
        self.cylinder_x_inner(center, radius, x, None)
    }

    fn cylinder_x_inner(
        &mut self,
        center: (i32, i32),
        radius: f32,
        x: (i32, i32),
        color: Option<u8>,
    ) -> &mut Self {
        let r2 = radius * radius;
        let span = radius.ceil() as i32;
        for xx in x.0..=x.1 {
            for dz in -span..=span {
                for dy in -span..=span {
                    if (dy * dy + dz * dz) as f32 <= r2 {
                        match color {
                            Some(c) => self.set(xx, center.0 + dy, center.1 + dz, c),
                            None => self.erase(xx, center.0 + dy, center.1 + dz),
                        }
                    }
                }
            }
        }
        self
    }

    /// Solid cylinder whose axis runs along +Y (a belt roller, a cross-pipe).
    pub fn cylinder_y(
        &mut self,
        center: (i32, i32),
        radius: f32,
        y: (i32, i32),
        color: u8,
    ) -> &mut Self {
        self.cylinder_y_inner(center, radius, y, Some(color))
    }

    /// The bore: [`Self::cylinder_y`] in reverse, leaving empty space — the
    /// Y-axis twin of [`Self::carve_cylinder_x`], added for the mixer's
    /// horizontal ribbon drum, which lies along +Y because its footprint
    /// does.
    pub fn carve_cylinder_y(
        &mut self,
        center: (i32, i32),
        radius: f32,
        y: (i32, i32),
    ) -> &mut Self {
        self.cylinder_y_inner(center, radius, y, None)
    }

    fn cylinder_y_inner(
        &mut self,
        center: (i32, i32),
        radius: f32,
        y: (i32, i32),
        color: Option<u8>,
    ) -> &mut Self {
        let r2 = radius * radius;
        let span = radius.ceil() as i32;
        for yy in y.0..=y.1 {
            for dz in -span..=span {
                for dx in -span..=span {
                    if (dx * dx + dz * dz) as f32 <= r2 {
                        match color {
                            Some(c) => self.set(center.0 + dx, yy, center.1 + dz, c),
                            None => self.erase(center.0 + dx, yy, center.1 + dz),
                        }
                    }
                }
            }
        }
        self
    }

    /// Solid cylinder along +X whose axis may sit BETWEEN voxels — the
    /// fractional-centre twin of [`Self::cylinder_x`], added for the pipe
    /// family ("Pipes & Conveyors Overhaul", ticket 3). `center` is in
    /// voxel-index units (voxel `i` is centred on `i`), so a Y centre of
    /// 15.5 puts the axis exactly on the tile's midline between voxels 15
    /// and 16. That is what lets a tile turned 90° or 180° about its
    /// footprint centre land on the same voxels as its neighbour; an
    /// integer centre (16) is half a voxel off the midline and a run of
    /// tiles at mixed facings steps by one voxel at every joint.
    pub fn cylinder_x_f(&mut self, center: (f32, f32), radius: f32, x: (i32, i32), color: u8) -> &mut Self {
        self.cylinder_x_f_inner(center, radius, x, Some(color))
    }

    /// [`Self::cylinder_x_f`] in reverse.
    pub fn carve_cylinder_x_f(&mut self, center: (f32, f32), radius: f32, x: (i32, i32)) -> &mut Self {
        self.cylinder_x_f_inner(center, radius, x, None)
    }

    fn cylinder_x_f_inner(&mut self, center: (f32, f32), radius: f32, x: (i32, i32), color: Option<u8>) -> &mut Self {
        let r2 = radius * radius;
        let span = (radius + 1.0).ceil() as i32;
        let (cy, cz) = (center.0.round() as i32, center.1.round() as i32);
        for xx in x.0..=x.1 {
            for zz in (cz - span)..=(cz + span) {
                for yy in (cy - span)..=(cy + span) {
                    let (dy, dz) = (yy as f32 - center.0, zz as f32 - center.1);
                    if dy * dy + dz * dz <= r2 {
                        match color {
                            Some(c) => self.set(xx, yy, zz, c),
                            None => self.erase(xx, yy, zz),
                        }
                    }
                }
            }
        }
        self
    }

    /// The +Y twin of [`Self::cylinder_x_f`]: `center` is `(x, z)`.
    pub fn cylinder_y_f(&mut self, center: (f32, f32), radius: f32, y: (i32, i32), color: u8) -> &mut Self {
        self.cylinder_y_f_inner(center, radius, y, Some(color))
    }

    /// [`Self::cylinder_y_f`] in reverse.
    pub fn carve_cylinder_y_f(&mut self, center: (f32, f32), radius: f32, y: (i32, i32)) -> &mut Self {
        self.cylinder_y_f_inner(center, radius, y, None)
    }

    fn cylinder_y_f_inner(&mut self, center: (f32, f32), radius: f32, y: (i32, i32), color: Option<u8>) -> &mut Self {
        let r2 = radius * radius;
        let span = (radius + 1.0).ceil() as i32;
        let (cx, cz) = (center.0.round() as i32, center.1.round() as i32);
        for yy in y.0..=y.1 {
            for zz in (cz - span)..=(cz + span) {
                for xx in (cx - span)..=(cx + span) {
                    let (dx, dz) = (xx as f32 - center.0, zz as f32 - center.1);
                    if dx * dx + dz * dz <= r2 {
                        match color {
                            Some(c) => self.set(xx, yy, zz, c),
                            None => self.erase(xx, yy, zz),
                        }
                    }
                }
            }
        }
        self
    }

    /// The +Z twin, for a hand wheel or a hub whose centre is on the tile
    /// midline: `center` is `(x, y)`.
    pub fn cylinder_z_f(&mut self, center: (f32, f32), radius: f32, z: (i32, i32), color: u8) -> &mut Self {
        self.cylinder_z_f_inner(center, radius, z, Some(color))
    }

    /// [`Self::cylinder_z_f`] in reverse.
    pub fn carve_cylinder_z_f(&mut self, center: (f32, f32), radius: f32, z: (i32, i32)) -> &mut Self {
        self.cylinder_z_f_inner(center, radius, z, None)
    }

    fn cylinder_z_f_inner(&mut self, center: (f32, f32), radius: f32, z: (i32, i32), color: Option<u8>) -> &mut Self {
        let r2 = radius * radius;
        let span = (radius + 1.0).ceil() as i32;
        let (cx, cy) = (center.0.round() as i32, center.1.round() as i32);
        for zz in z.0..=z.1 {
            for yy in (cy - span)..=(cy + span) {
                for xx in (cx - span)..=(cx + span) {
                    let (dx, dy) = (xx as f32 - center.0, yy as f32 - center.1);
                    if dx * dx + dy * dy <= r2 {
                        match color {
                            Some(c) => self.set(xx, yy, zz, c),
                            None => self.erase(xx, yy, zz),
                        }
                    }
                }
            }
        }
        self
    }

    /// A ball with a fractional centre — the cast body where two pipe legs
    /// meet. Same coordinate rule as [`Self::cylinder_x_f`].
    pub fn sphere_f(&mut self, center: (f32, f32, f32), radius: f32, color: u8) -> &mut Self {
        self.sphere_f_inner(center, radius, Some(color))
    }

    /// [`Self::sphere_f`] in reverse: the hollow inside that cast body.
    pub fn carve_sphere_f(&mut self, center: (f32, f32, f32), radius: f32) -> &mut Self {
        self.sphere_f_inner(center, radius, None)
    }

    fn sphere_f_inner(&mut self, center: (f32, f32, f32), radius: f32, color: Option<u8>) -> &mut Self {
        let r2 = radius * radius;
        let span = (radius + 1.0).ceil() as i32;
        let (cx, cy, cz) = (center.0.round() as i32, center.1.round() as i32, center.2.round() as i32);
        for zz in (cz - span)..=(cz + span) {
            for yy in (cy - span)..=(cy + span) {
                for xx in (cx - span)..=(cx + span) {
                    let (dx, dy, dz) = (xx as f32 - center.0, yy as f32 - center.1, zz as f32 - center.2);
                    if dx * dx + dy * dy + dz * dz <= r2 {
                        match color {
                            Some(c) => self.set(xx, yy, zz, c),
                            None => self.erase(xx, yy, zz),
                        }
                    }
                }
            }
        }
        self
    }

    /// Draws every voxel of `other` into this model (later wins), clipped to
    /// this model's volume. This is how a composable fitting is proved in
    /// the scene before Godot overlays it as its own node: the deprecated
    /// `pipe_source`/`pipe_inlet` are a shape plus a fitting merged here.
    pub fn merge(&mut self, other: &VoxModel) -> &mut Self {
        for (&(x, y, z), &idx) in &other.voxels {
            self.set(x as i32, y as i32, z as i32, idx);
        }
        self
    }

    /// This model turned `turns` quarter turns CLOCKWISE (seen from above)
    /// about its footprint centre — the same turn `Direction::rotate_cw`
    /// applies and the same one Godot applies to a placement, so what this
    /// returns is exactly what the scene shows for a rotated tile. In vox
    /// coordinates that is `(dx, dy) -> (-dy, dx)`: the +X face goes to
    /// the +Y (south) face. Requires a square footprint.
    pub fn rotated_cw(&self, turns: u32) -> VoxModel {
        assert_eq!(self.size[0], self.size[1], "rotated_cw needs a square footprint");
        let mut out = self.clone();
        let last = self.size[0] - 1;
        for _ in 0..(turns % 4) {
            let mut next = VoxModel::new(out.size);
            for (&(x, y, z), &idx) in &out.voxels {
                next.voxels.insert((last - y, x, z), idx);
            }
            out = next;
        }
        out
    }

    /// Axis-aligned ellipsoid — tomatoes, domed tank tops, bottle shoulders.
    pub fn ellipsoid(
        &mut self,
        center: (i32, i32, i32),
        radii: (f32, f32, f32),
        color: u8,
    ) -> &mut Self {
        let span = (
            radii.0.ceil() as i32,
            radii.1.ceil() as i32,
            radii.2.ceil() as i32,
        );
        for dz in -span.2..=span.2 {
            for dy in -span.1..=span.1 {
                for dx in -span.0..=span.0 {
                    let n = (dx as f32 / radii.0).powi(2)
                        + (dy as f32 / radii.1).powi(2)
                        + (dz as f32 / radii.2).powi(2);
                    if n <= 1.0 {
                        self.set(center.0 + dx, center.1 + dy, center.2 + dz, color);
                    }
                }
            }
        }
        self
    }

    /// Alternating hazard striping across X (UI_UX.md §3's "real industrial
    /// visual language" — signage, not decoration).
    pub fn hazard_stripe_x(
        &mut self,
        x: (i32, i32),
        y: (i32, i32),
        z: (i32, i32),
        period: i32,
        a: u8,
        b: u8,
    ) -> &mut Self {
        for zz in z.0..=z.1 {
            for yy in y.0..=y.1 {
                for xx in x.0..=x.1 {
                    let color = if ((xx + zz).rem_euclid(period * 2)) < period { a } else { b };
                    self.set(xx, yy, zz, color);
                }
            }
        }
        self
    }

    /// The subset of this model whose voxels belong to `class`, as its own
    /// model of identical size. This is how one asset becomes one mesh per
    /// material class in Godot (see this module's doc and main.gd) — the
    /// mechanism behind UI_UX.md §4's "metals get a deliberate exception."
    pub fn filter_class(&self, palette: &Palette, class: MaterialClass) -> VoxModel {
        let mut out = VoxModel::new(self.size);
        for (&pos, &idx) in &self.voxels {
            if palette.class_of(idx) == class {
                out.voxels.insert(pos, idx);
            }
        }
        out
    }

    pub fn is_empty(&self) -> bool {
        self.voxels.is_empty()
    }
}

// ---------------------------------------------------------------------------
// .vox encoding
// ---------------------------------------------------------------------------

fn chunk(id: &[u8; 4], content: Vec<u8>, children: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + content.len() + children.len());
    out.extend_from_slice(id);
    out.extend_from_slice(&(content.len() as u32).to_le_bytes());
    out.extend_from_slice(&(children.len() as u32).to_le_bytes());
    out.extend_from_slice(&content);
    out.extend_from_slice(&children);
    out
}

fn vox_string(s: &str) -> Vec<u8> {
    let mut out = (s.len() as u32).to_le_bytes().to_vec();
    out.extend_from_slice(s.as_bytes());
    out
}

fn vox_dict(pairs: &[(&str, String)]) -> Vec<u8> {
    let mut out = (pairs.len() as u32).to_le_bytes().to_vec();
    for (k, v) in pairs {
        out.extend_from_slice(&vox_string(k));
        out.extend_from_slice(&vox_string(v));
    }
    out
}

/// Serializes `model` as a single-model `.vox` file.
///
/// Emits, in the order MagicaVoxel itself does: `SIZE`, `XYZI`, the minimal
/// scene graph (`nTRN` -> `nGRP` -> `nTRN` -> `nSHP`) the Godot importer
/// requires, `LAYR`, `RGBA`, and one `MATL` per palette entry.
///
/// The inner `nTRN` translates the model up by half its height, because the
/// importer re-centres each model on its own volume: without this the mesh's
/// origin would sit at the model's middle, and every asset in the scene
/// would need a per-asset vertical fudge factor in GDScript. With it, a
/// model's origin is its footprint centre at ground level.
pub fn encode(model: &VoxModel, palette: &Palette) -> Vec<u8> {
    let mut children = Vec::new();

    // SIZE
    let mut size = Vec::new();
    for v in model.size {
        size.extend_from_slice(&v.to_le_bytes());
    }
    children.extend_from_slice(&chunk(b"SIZE", size, Vec::new()));

    // XYZI. MagicaVoxel stores each coordinate as a single byte, so this is
    // also the hard ceiling on model size (256^3) — far beyond §2's 32^3.
    let mut xyzi = (model.voxels.len() as u32).to_le_bytes().to_vec();
    for (&(x, y, z), &idx) in &model.voxels {
        xyzi.extend_from_slice(&[x as u8, y as u8, z as u8, idx]);
    }
    children.extend_from_slice(&chunk(b"XYZI", xyzi, Vec::new()));

    // Scene graph: root transform -> group -> shape transform -> shape.
    let mut ntrn_root = 0u32.to_le_bytes().to_vec();
    ntrn_root.extend_from_slice(&vox_dict(&[]));
    ntrn_root.extend_from_slice(&1u32.to_le_bytes()); // child node id
    ntrn_root.extend_from_slice(&(-1i32).to_le_bytes()); // reserved
    ntrn_root.extend_from_slice(&(-1i32).to_le_bytes()); // layer id
    ntrn_root.extend_from_slice(&1u32.to_le_bytes()); // frame count
    ntrn_root.extend_from_slice(&vox_dict(&[]));
    children.extend_from_slice(&chunk(b"nTRN", ntrn_root, Vec::new()));

    let mut ngrp = 1u32.to_le_bytes().to_vec();
    ngrp.extend_from_slice(&vox_dict(&[]));
    ngrp.extend_from_slice(&1u32.to_le_bytes()); // child count
    ngrp.extend_from_slice(&2u32.to_le_bytes()); // child node id
    children.extend_from_slice(&chunk(b"nGRP", ngrp, Vec::new()));

    let lift = (model.size[2] / 2) as i32;
    let mut ntrn_shape = 2u32.to_le_bytes().to_vec();
    ntrn_shape.extend_from_slice(&vox_dict(&[]));
    ntrn_shape.extend_from_slice(&3u32.to_le_bytes());
    ntrn_shape.extend_from_slice(&(-1i32).to_le_bytes());
    ntrn_shape.extend_from_slice(&0i32.to_le_bytes()); // layer 0
    ntrn_shape.extend_from_slice(&1u32.to_le_bytes());
    ntrn_shape.extend_from_slice(&vox_dict(&[("_t", format!("0 0 {lift}"))]));
    children.extend_from_slice(&chunk(b"nTRN", ntrn_shape, Vec::new()));

    let mut nshp = 3u32.to_le_bytes().to_vec();
    nshp.extend_from_slice(&vox_dict(&[]));
    nshp.extend_from_slice(&1u32.to_le_bytes()); // model count
    nshp.extend_from_slice(&0u32.to_le_bytes()); // model id
    nshp.extend_from_slice(&vox_dict(&[]));
    children.extend_from_slice(&chunk(b"nSHP", nshp, Vec::new()));

    let mut layr = 0u32.to_le_bytes().to_vec();
    layr.extend_from_slice(&vox_dict(&[("_name", "asset".into()), ("_hidden", "0".into())]));
    layr.extend_from_slice(&(-1i32).to_le_bytes());
    children.extend_from_slice(&chunk(b"LAYR", layr, Vec::new()));

    // RGBA: 256 entries; entry i holds the colour of palette index i + 1.
    let mut rgba = Vec::with_capacity(1024);
    for i in 0..256u32 {
        let idx = (i + 1) as u8;
        match palette.rgb_of(idx) {
            Some([r, g, b]) => rgba.extend_from_slice(&[r, g, b, 255]),
            None => rgba.extend_from_slice(&[0, 0, 0, 255]),
        }
    }
    children.extend_from_slice(&chunk(b"RGBA", rgba, Vec::new()));

    // MATL, one per defined colour. Nothing in the Godot path reads these
    // (main.gd builds its materials from materials.json instead, so the
    // numbers stay in palette.ron rather than being duplicated in a binary),
    // but writing them keeps the .vox honest for vengi and MagicaVoxel.
    for entry in palette.entries() {
        let props: Vec<(&str, String)> = match entry.class {
            MaterialClass::Metal => vec![
                ("_type", "_metal".into()),
                ("_weight", format!("{:.3}", palette.material(entry.class).metallic)),
                ("_rough", format!("{:.3}", palette.material(entry.class).roughness)),
                ("_spec", "0.900".into()),
                ("_ior", "0.300".into()),
            ],
            MaterialClass::Emissive => vec![
                ("_type", "_emit".into()),
                ("_weight", "1.000".into()),
                ("_flux", format!("{:.3}", palette.material(entry.class).emission_energy)),
            ],
            // A liquid is written as a plain diffuse material with its own
            // roughness: MagicaVoxel's `_glass`/`_media` types describe
            // refractive volumes, which is not what this is (the fluid body
            // is opaque geometry Godot tints and gloss-shades). Nothing in
            // the Godot path reads MATL anyway — see this block's comment.
            MaterialClass::Matte | MaterialClass::Liquid => vec![
                ("_type", "_diffuse".into()),
                ("_weight", "1.000".into()),
                ("_rough", format!("{:.3}", palette.material(entry.class).roughness)),
            ],
        };
        let mut matl = (entry.index as u32).to_le_bytes().to_vec();
        matl.extend_from_slice(&vox_dict(&props));
        children.extend_from_slice(&chunk(b"MATL", matl, Vec::new()));
    }

    let mut out = b"VOX ".to_vec();
    out.extend_from_slice(&150u32.to_le_bytes());
    out.extend_from_slice(&chunk(b"MAIN", Vec::new(), children));
    out
}

pub fn write_vox(path: &Path, model: &VoxModel, palette: &Palette) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let bytes = encode(model, palette);
    let mut f = std::fs::File::create(path)?;
    f.write_all(&bytes)?;
    Ok(())
}
