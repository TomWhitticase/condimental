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
}

impl MaterialClass {
    pub fn name(self) -> &'static str {
        match self {
            MaterialClass::Matte => "matte",
            MaterialClass::Metal => "metal",
            MaterialClass::Emissive => "emissive",
        }
    }

    /// Parsed from palette.ron's `class:` field. Kept as a plain string in
    /// the file (rather than a RON enum literal) so the palette reads as
    /// data a non-Rust tool could also consume.
    pub fn from_name(name: &str) -> Option<MaterialClass> {
        MaterialClass::all().into_iter().find(|c| c.name() == name)
    }

    pub fn all() -> [MaterialClass; 3] {
        [MaterialClass::Matte, MaterialClass::Metal, MaterialClass::Emissive]
    }
}

#[derive(Debug, Clone, Copy, Deserialize, serde::Serialize)]
pub struct MaterialDef {
    pub metallic: f32,
    pub roughness: f32,
    pub emission_energy: f32,
}

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
}

/// Only schema version this loader understands (ARCHITECTURE.md §4: reject a
/// mismatch rather than silently misreading a changed shape).
const SUPPORTED_SCHEMA_VERSION: u32 = 1;

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
            let def = file
                .materials
                .get(class.name())
                .unwrap_or_else(|| panic!("palette.ron is missing material class '{}'", class.name()));
            materials.insert(class, *def);
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
                    panic!("palette colour '{}' has unknown class '{}'", row.name, row.class)
                }),
            });
        }

        Self { entries, by_name, materials }
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
        self.entries.iter().find(|e| e.index == index).map(|e| e.rgb)
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
}
