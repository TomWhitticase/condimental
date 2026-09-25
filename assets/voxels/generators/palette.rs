//! Loads `assets/voxels/palette.ron` — the shared, checked-in colour palette
//! every generator references (UI_UX.md §2a point 5).
//!
//! Embedded with `include_str!` rather than read at runtime, matching how
//! `sim_core` embeds `data/recipes.ron`: the generator then resolves its
//! content identically no matter what directory it is invoked from.

use std::collections::BTreeMap;

use serde::Deserialize;

const PALETTE_RON: &str = include_str!("../palette.ron");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MaterialClass {
    Matte,
    Metal,
    Emissive,
    /// Liquid surfaces (milestone 6's fluids): a dielectric like `Matte`,
    /// but markedly glossier — UI_UX.md §4 asks for "visible liquid
    /// surfaces in tanks/pipes with subtle shading/reflection", which is a
    /// material property, so it belongs here rather than as a per-asset
    /// decision in GDScript.
    Liquid,
}

impl MaterialClass {
    pub fn name(self) -> &'static str {
        match self {
            MaterialClass::Matte => "matte",
            MaterialClass::Metal => "metal",
            MaterialClass::Emissive => "emissive",
            MaterialClass::Liquid => "liquid",
        }
    }

    /// Parsed from palette.ron's `class:` field. Kept as a plain string in
    /// the file (rather than a RON enum literal) so the palette reads as
    /// data a non-Rust tool could also consume.
    pub fn from_name(name: &str) -> Option<MaterialClass> {
        MaterialClass::all().into_iter().find(|c| c.name() == name)
    }

    pub fn all() -> [MaterialClass; 4] {
        [
            MaterialClass::Matte,
            MaterialClass::Metal,
            MaterialClass::Emissive,
            MaterialClass::Liquid,
        ]
    }
}

#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize)]
pub struct MaterialDef {
    pub metallic: f32,
    pub roughness: f32,
    pub emission_energy: f32,
}

/// One state a status lamp is drawn in, exactly as palette.ron writes it: a
/// MODIFIER of the `emissive` class, never a second copy of it. That is what
/// keeps `emission_energy` written down once — retuning the class moves the
/// lit lamp with it, and no renderer can be left holding a stale number.
#[derive(Debug, Clone, Copy, Deserialize)]
pub struct LampStateDef {
    pub roughness: f32,
    /// Multiplied into the `emissive` class's `emission_energy`. Zero on the
    /// idle state: an unlit lens that still emitted would be a lamp half on.
    pub emission_scale: f32,
    /// Multiplied into the lens's own colour in the ALBEDO channel alone.
    pub albedo_scale: f32,
}

/// A lamp state RESOLVED against the `emissive` class — absolute numbers a
/// renderer can hand straight to a material with no palette knowledge at all.
/// This is what `lamp_states.json` carries, and it is the only thing
/// `voxel_assets.gd` and `tools/asset-browser` read.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct LampMaterialDef {
    pub metallic: f32,
    pub roughness: f32,
    pub emission_energy: f32,
    pub albedo_scale: f32,
}

/// The states every palette must define. Not an open set: `main.gd` swaps
/// between exactly these two, so a palette missing one is a build error
/// rather than a lamp that silently never changes.
pub const LAMP_STATES: [&str; 2] = ["idle", "running"];

#[derive(Debug, Deserialize)]
struct ColorRow {
    name: String,
    rgb: String,
    class: String,
}

#[derive(Debug, Deserialize)]
struct PaletteFile {
    schema_version: u32,
    materials: BTreeMap<String, MaterialDef>,
    lamp_states: BTreeMap<String, LampStateDef>,
    colors: Vec<ColorRow>,
}

#[derive(Debug, Clone)]
pub struct PaletteEntry {
    /// Kept for diagnostics/tooling even though the encoder addresses
    /// colours by index — a nameless palette entry is much harder to debug.
    #[allow(dead_code)]
    pub name: String,
    /// 1-based MagicaVoxel palette index, derived from list order.
    pub index: u8,
    pub rgb: [u8; 3],
    pub class: MaterialClass,
}

pub struct Palette {
    entries: Vec<PaletteEntry>,
    by_name: BTreeMap<String, u8>,
    materials: BTreeMap<MaterialClass, MaterialDef>,
    /// Resolved at load, so nothing downstream has to know that palette.ron
    /// wrote these as modifiers.
    lamp_states: BTreeMap<String, LampMaterialDef>,
}

/// Only schema version this loader understands (ARCHITECTURE.md §4: reject a
/// mismatch rather than silently misreading a changed shape).
const SUPPORTED_SCHEMA_VERSION: u32 = 2;

fn parse_hex(s: &str) -> [u8; 3] {
    let s = s.trim_start_matches('#');
    assert_eq!(s.len(), 6, "palette rgb must be #RRGGBB, got '{s}'");
    let byte = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).expect("palette rgb must be hex");
    [byte(0), byte(2), byte(4)]
}

impl Palette {
    pub fn load() -> Self {
        let file: PaletteFile =
            ron::from_str(PALETTE_RON).expect("assets/voxels/palette.ron must parse");
        assert_eq!(
            file.schema_version, SUPPORTED_SCHEMA_VERSION,
            "palette.ron schema_version {} is not supported (expected {SUPPORTED_SCHEMA_VERSION})",
            file.schema_version
        );
        assert!(
            file.colors.len() <= 255,
            "palette.ron has {} colours; MagicaVoxel palettes hold at most 255",
            file.colors.len()
        );

        let mut materials = BTreeMap::new();
        for class in MaterialClass::all() {
            let def = file.materials.get(class.name()).unwrap_or_else(|| {
                panic!("palette.ron is missing material class '{}'", class.name())
            });
            materials.insert(class, *def);
        }

        let emissive = materials[&MaterialClass::Emissive];
        let mut lamp_states = BTreeMap::new();
        for state in LAMP_STATES {
            let def = file
                .lamp_states
                .get(state)
                .unwrap_or_else(|| panic!("palette.ron is missing lamp state '{state}'"));
            lamp_states.insert(
                state.to_string(),
                LampMaterialDef {
                    metallic: emissive.metallic,
                    roughness: def.roughness,
                    emission_energy: emissive.emission_energy * def.emission_scale,
                    albedo_scale: def.albedo_scale,
                },
            );
        }

        let mut entries = Vec::new();
        let mut by_name = BTreeMap::new();
        for (i, row) in file.colors.iter().enumerate() {
            let index = (i + 1) as u8;
            assert!(
                by_name.insert(row.name.clone(), index).is_none(),
                "duplicate palette colour name '{}'",
                row.name
            );
            entries.push(PaletteEntry {
                name: row.name.clone(),
                index,
                rgb: parse_hex(&row.rgb),
                class: MaterialClass::from_name(&row.class).unwrap_or_else(|| {
                    panic!(
                        "palette colour '{}' has unknown class '{}'",
                        row.name, row.class
                    )
                }),
            });
        }

        Self {
            entries,
            by_name,
            materials,
            lamp_states,
        }
    }

    /// Palette index for a colour name. Panics on an unknown name — a
    /// generator naming a colour that isn't in palette.ron is a mistake to
    /// surface at build time, the same way an unknown recipe id is.
    pub fn idx(&self, name: &str) -> u8 {
        *self
            .by_name
            .get(name)
            .unwrap_or_else(|| panic!("no colour named '{name}' in assets/voxels/palette.ron"))
    }

    pub fn entries(&self) -> &[PaletteEntry] {
        &self.entries
    }

    pub fn rgb_of(&self, index: u8) -> Option<[u8; 3]> {
        self.entries
            .iter()
            .find(|e| e.index == index)
            .map(|e| e.rgb)
    }

    pub fn class_of(&self, index: u8) -> MaterialClass {
        self.entries
            .iter()
            .find(|e| e.index == index)
            .map(|e| e.class)
            .unwrap_or(MaterialClass::Matte)
    }

    pub fn material(&self, class: MaterialClass) -> MaterialDef {
        self.materials[&class]
    }

    /// The material table Godot rebuilds its `StandardMaterial3D`s from, so
    /// palette.ron stays the only place these numbers are written down.
    pub fn materials_json(&self) -> String {
        let map: BTreeMap<&str, MaterialDef> = MaterialClass::all()
            .into_iter()
            .map(|c| (c.name(), self.materials[&c]))
            .collect();
        serde_json::to_string_pretty(&map).expect("material table must serialize")
    }

    /// The same, for the two states a status lamp is drawn in (issue #81).
    /// Resolved against the `emissive` class, so a consumer applies these
    /// numbers as they stand and two renderers cannot disagree about what an
    /// idle lamp looks like: `godot_project/voxel_assets.gd` builds
    /// `lamp_idle`/`lamp_running` from this file, and the asset browser's
    /// running/idle toggle reads the very same one.
    pub fn lamp_states_json(&self) -> String {
        serde_json::to_string_pretty(&self.lamp_states).expect("lamp states must serialize")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The status-lamp states are the `emissive` class MODIFIED, and the
    /// exported table has to prove it rather than carry its own numbers: a
    /// lit lamp emits exactly the energy the class asks for (so retuning the
    /// class retunes the lamp), an idle one emits nothing at all, and the
    /// idle lens is visibly darker in ALBEDO — the channel that still renders
    /// when the glow pass is what the eye is reading. `voxel_assets.gd` and
    /// `tools/asset-browser` both build their materials straight from this.
    #[test]
    fn lamp_states_resolve_against_the_emissive_class() {
        let palette = Palette::load();
        let emissive = palette.material(MaterialClass::Emissive);
        let json: serde_json::Value =
            serde_json::from_str(&palette.lamp_states_json()).expect("lamp states must be JSON");

        for state in LAMP_STATES {
            assert!(
                json.get(state).is_some(),
                "lamp_states.json must carry '{state}'"
            );
        }
        let field = |state: &str, key: &str| json[state][key].as_f64().expect("number") as f32;

        assert_eq!(
            field("running", "emission_energy"),
            emissive.emission_energy,
            "a lit lamp emits the `emissive` class's own energy, never a second copy of it"
        );
        assert_eq!(
            field("idle", "emission_energy"),
            0.0,
            "an unlit lens that still emitted would be a lamp half on"
        );
        assert!(
            field("idle", "albedo_scale") < field("running", "albedo_scale") * 0.6,
            "an idle lens must be visibly darker in ALBEDO, not merely dimmer in a channel \
             a glow-lit frame does not show"
        );
        assert_eq!(
            field("idle", "metallic"),
            emissive.metallic,
            "a lamp state modifies the `emissive` class, so metallic is the class's"
        );
    }
}
