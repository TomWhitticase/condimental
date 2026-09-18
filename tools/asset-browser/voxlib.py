"""Reads the committed voxel pipeline output for the asset browser.

Everything here is derived from files already in the repo — `models/*.vox`,
`models/index.json`, `palette.ron` and the generators themselves — so the
browser can never disagree with what `cargo run -p voxgen` last wrote. There
is no build step and no second copy of any asset fact.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass, field
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
VOXELS = REPO / "assets" / "voxels"
MODELS = VOXELS / "models"
ICONS = VOXELS / "icons"
PREVIEWS = VOXELS / "previews"
GENERATORS = VOXELS / "generators"
MAIN_GD = REPO / "godot_project" / "main.gd"

TILE_VOXELS = 32


# --------------------------------------------------------------------------
# .vox parsing (the hand-written encoder in generators/vox.rs, read back)
# --------------------------------------------------------------------------


def _chunks(buf: bytes, pos: int, end: int):
    while pos + 12 <= end:
        cid = buf[pos : pos + 4]
        n = int.from_bytes(buf[pos + 4 : pos + 8], "little")
        m = int.from_bytes(buf[pos + 12 - 4 : pos + 12], "little")
        content = buf[pos + 12 : pos + 12 + n]
        yield cid, content, pos + 12 + n, pos + 12 + n + m
        pos += 12 + n + m


def parse_vox(path: Path) -> dict:
    """One `.vox` file -> {size, voxels (flat x,y,z,idx), palette (hex)}.

    voxgen writes exactly one model per file, so the `nTRN`/`nGRP`/`nSHP`
    scene graph carries no information the viewer needs — only the lift the
    Godot importer undoes anyway. SIZE, XYZI and RGBA are the whole story.
    """
    buf = path.read_bytes()
    if buf[:4] != b"VOX ":
        raise ValueError(f"{path.name} is not a .vox file")
    size = [0, 0, 0]
    voxels: list[int] = []
    palette: list[str] = []
    for cid, content, cstart, cend in _chunks(buf, 8, len(buf)):
        if cid == b"MAIN":
            for c2, body, _, _ in _chunks(buf, cstart, cend):
                if c2 == b"SIZE":
                    size = [
                        int.from_bytes(body[i : i + 4], "little") for i in (0, 4, 8)
                    ]
                elif c2 == b"XYZI":
                    count = int.from_bytes(body[0:4], "little")
                    for i in range(count):
                        x, y, z, idx = body[4 + i * 4 : 8 + i * 4]
                        voxels += [x, y, z, idx]
                elif c2 == b"RGBA":
                    palette = [
                        "#%02x%02x%02x" % (body[i], body[i + 1], body[i + 2])
                        for i in range(0, 1024, 4)
                    ]
    return {"size": size, "voxels": voxels, "palette": palette}


def model_stats(parsed: dict) -> dict:
    """Bounding box and per-palette-index voxel counts."""
    vx = parsed["voxels"]
    if not vx:
        return {"count": 0, "bbox": None, "colors": {}}
    lo = [255, 255, 255]
    hi = [0, 0, 0]
    colors: dict[int, int] = {}
    for i in range(0, len(vx), 4):
        for a in range(3):
            lo[a] = min(lo[a], vx[i + a])
            hi[a] = max(hi[a], vx[i + a])
        idx = vx[i + 3]
        colors[idx] = colors.get(idx, 0) + 1
    return {
        "count": len(vx) // 4,
        "bbox": {"min": lo, "max": hi, "size": [hi[a] - lo[a] + 1 for a in range(3)]},
        "colors": colors,
    }


# --------------------------------------------------------------------------
# palette.ron
# --------------------------------------------------------------------------

_COLOR_RE = re.compile(
    r'\(\s*name:\s*"([^"]+)"\s*,\s*rgb:\s*"([^"]+)"\s*,\s*class:\s*"([^"]+)"\s*\)'
)


def load_palette() -> list[dict]:
    """Palette entries in file order; index i is MagicaVoxel index i + 1."""
    text = (VOXELS / "palette.ron").read_text()
    body = text.split("colors:", 1)[1]
    out = []
    for i, (name, rgb, cls) in enumerate(_COLOR_RE.findall(body)):
        out.append({"index": i + 1, "name": name, "rgb": rgb, "class": cls})
    return out


# --------------------------------------------------------------------------
# generators/assets.rs — the source of truth, mined for prose
# --------------------------------------------------------------------------

_ASSET_RE = re.compile(
    r'Asset\s*\{\s*name:\s*"([^"]+)"\s*,\s*build:\s*(\w+)\s*,?\s*\}', re.S
)


def _doc_above(lines: list[str], line_no: int) -> str:
    """The `///` block immediately above a `fn`, as plain markdown."""
    out = []
    i = line_no - 2  # 0-based index of the line above the fn
    while i >= 0:
        s = lines[i].strip()
        if s.startswith("///"):
            out.append(s[3:].lstrip() if len(s) > 3 else "")
            i -= 1
        elif s.startswith("#["):  # attribute between doc and fn
            i -= 1
        else:
            break
    return "\n".join(reversed(out)).strip()


def _note_above(lines: list[str], line_no: int) -> str:
    """The `//` registry comment above an `ASSETS` entry — family context."""
    out = []
    i = line_no - 2
    while i >= 0:
        s = lines[i].strip()
        if s.startswith("//") and not s.startswith("///"):
            out.append(s[2:].lstrip())
            i -= 1
        else:
            break
    return " ".join(reversed(out)).strip()


def _macro_aliases(text: str) -> dict[str, tuple[str, str]]:
    """Resolve macro-generated generators to the function that does the work.

    Frame variants are declared as `belt_f1 => BeltShape::Straight, 1;`
    inside a `macro_rules!` block whose body calls one real builder. The
    prose worth reading lives on that builder, so map each generated name
    onto it — and keep the invocation line as the variant's own caption.
    """
    macros: dict[str, str | None] = {}
    for m in re.finditer(r"macro_rules!\s*(\w+)\s*\{(.*?)\n\}", text, re.S):
        call = re.search(r"->\s*VoxModel\s*\{\s*(\$?\w+)\s*\(", m.group(2))
        macros[m.group(1)] = call.group(1) if call else None

    out: dict[str, tuple[str, str]] = {}
    for m in re.finditer(r"^(\w+)!\s*\{(.*?)\n\}", text, re.S | re.M):
        call = macros.get(m.group(1))
        if call is None:
            continue
        for line in m.group(2).splitlines():
            entry = re.match(r"\s*(\w+)\s*=>\s*(.+?)\s*;\s*$", line)
            if not entry:
                continue
            name, args = entry.group(1), entry.group(2)
            if call.startswith("$"):
                first = re.match(r"(\w+)", args)
                if not first:
                    continue
                target = first.group(1)
            else:
                target = call
            out[name] = (target, f"{m.group(1)}! {{ {name} => {args}; }}")
    return out


def load_generators() -> dict[str, dict]:
    """asset name -> {fn, file, line, doc, note} from the Rust registry."""
    out: dict[str, dict] = {}
    for src in sorted(GENERATORS.glob("*.rs")):
        text = src.read_text()
        lines = text.splitlines()
        offsets = [0]
        for ln in lines:
            offsets.append(offsets[-1] + len(ln) + 1)

        def line_of(pos: int) -> int:
            lo, hi = 0, len(offsets) - 1
            while lo < hi:
                mid = (lo + hi) // 2
                if offsets[mid] <= pos:
                    lo = mid + 1
                else:
                    hi = mid
            return lo

        fn_line = {
            m.group(1): line_of(m.start())
            for m in re.finditer(r"^(?:pub )?fn (\w+)\(", text, re.M)
        }
        alias = _macro_aliases(text)
        for m in _ASSET_RE.finditer(text):
            name, fn = m.group(1), m.group(2)
            entry_line = line_of(m.start())
            via = None
            body_fn = fn
            if fn not in fn_line and fn in alias:
                body_fn, via = alias[fn]
            body_line = fn_line.get(body_fn)
            out[name] = {
                "fn": body_fn,
                "via": via,
                "file": str(src.relative_to(REPO)),
                "line": body_line or entry_line,
                "doc": _doc_above(lines, body_line) if body_line else "",
                "note": _note_above(lines, entry_line),
            }
    return out


# --------------------------------------------------------------------------
# main.gd — which game id draws which model
# --------------------------------------------------------------------------

_CONST_RE = re.compile(r"^const (\w+) :?=\s*\{", re.M)


def load_usage() -> dict[str, list[dict]]:
    """model name -> [{table, key}] harvested from main.gd's model tables."""
    if not MAIN_GD.exists():
        return {}
    usage: dict[str, list[dict]] = {}
    lines = MAIN_GD.read_text().splitlines()
    table = None
    for ln in lines:
        m = _CONST_RE.match(ln)
        if m:
            table = m.group(1)
            continue
        if table:
            if ln.startswith("}"):
                table = None
                continue
            kv = re.match(r'\s*"([^"]+)"\s*:\s*"([^"]+)"\s*,?', ln)
            if kv:
                usage.setdefault(kv.group(2), []).append(
                    {"table": table, "key": kv.group(1)}
                )
    # Single-model constants (fallbacks) matter too.
    for m in re.finditer(r'^const (\w+) :?=\s*"([^"]+)"\s*$', MAIN_GD.read_text(), re.M):
        if m.group(2) in usage or (MODELS / f"{m.group(2)}.vox").exists():
            usage.setdefault(m.group(2), []).append(
                {"table": m.group(1), "key": "(fallback)"}
            )
    return usage


# --------------------------------------------------------------------------
# data/machines.ron — footprints, so a model's tile count is checkable
# --------------------------------------------------------------------------


def load_footprints() -> dict[str, list[int]]:
    path = REPO / "data" / "machines.ron"
    if not path.exists():
        return {}
    out = {}
    text = path.read_text()
    for m in re.finditer(
        r'id:\s*"([^"]+)".*?Footprint\(\s*width_tiles:\s*(\d+)\s*,\s*height_tiles:\s*(\d+)',
        text,
        re.S,
    ):
        out.setdefault(m.group(1), [int(m.group(2)), int(m.group(3))])
    return out


# --------------------------------------------------------------------------
# Grouping: 79 models -> the handful of things a reviewer thinks in
# --------------------------------------------------------------------------

CATEGORY_RULES = [
    ("floor_tile", "Floor"),
    ("belt", "Belts"),
    ("pipe", "Pipes"),
    ("machine", "Machines"),
    ("splitter", "Machines"),
    ("item_", "Items"),
    ("staff_", "Staff"),
    ("hopper", "Storage"),
    ("fluid_tank", "Storage"),
]

CATEGORY_ORDER = [
    "Machines",
    "Belts",
    "Pipes",
    "Storage",
    "Items",
    "Floor",
    "Staff",
    "Misc",
]

_FRAME_RE = re.compile(r"^(.*)_f(\d+)$")

# An asset that is only ever drawn on top of another one tells a reviewer
# nothing on its own: `pipe_fluid` is a bare slab of liquid, the splitter's
# arrow is three lit voxels. The browser offers to draw its host behind it.
COMPANIONS = {"splitter_arrow": "machine_splitter"}


def _companion(asset_id: str, have: set[str]) -> str | None:
    if asset_id in COMPANIONS and COMPANIONS[asset_id] in have:
        return COMPANIONS[asset_id]
    if asset_id.startswith("pipe_fluid"):
        host = "pipe" + asset_id[len("pipe_fluid") :]
        if host in have:
            return host
    return None


def _category(name: str) -> str:
    for prefix, cat in CATEGORY_RULES:
        if name.startswith(prefix):
            return cat
    return "Misc"


def _label(name: str) -> str:
    return name.replace("_", " ")


@dataclass
class Variation:
    name: str
    role: str  # base | frame | motion | fill
    frame: int | None = None


@dataclass
class Asset:
    id: str
    category: str
    variations: list[Variation] = field(default_factory=list)


def group_models(names: list[str]) -> list[Asset]:
    """Fold model names into assets + their variations.

    Rules, derived from the naming the pipeline already uses:
      `X_f<n>`      animation frames of X          (belt, motion overlays)
      `X_motion`    the motion overlay of X        (cut out of X's body)
      `X_fill`      the fill overlay of X          (hopper, fluid tank)
    Anything else is an asset in its own right.
    """
    have = set(names)
    assets: dict[str, Asset] = {}

    def asset_for(key: str) -> Asset:
        if key not in assets:
            assets[key] = Asset(id=key, category=_category(key))
        return assets[key]

    for name in names:
        stem, frame = name, None
        m = _FRAME_RE.match(name)
        if m:
            stem, frame = m.group(1), int(m.group(2))
        if stem.endswith("_motion") and stem[: -len("_motion")] in have:
            host = stem[: -len("_motion")]
            role = "motion"
        elif stem.endswith("_fill") and stem[: -len("_fill")] in have:
            host = stem[: -len("_fill")]
            role = "fill"
        else:
            host = stem
            role = "frame" if frame is not None else "base"
        asset_for(host).variations.append(Variation(name=name, role=role, frame=frame))

    order = {c: i for i, c in enumerate(CATEGORY_ORDER)}
    return sorted(
        assets.values(), key=lambda a: (order.get(a.category, 99), a.id)
    )


# --------------------------------------------------------------------------
# The catalog the web app consumes
# --------------------------------------------------------------------------


def build_catalog() -> dict:
    index = json.loads((MODELS / "index.json").read_text())
    materials = json.loads((MODELS / "materials.json").read_text())
    palette = load_palette()
    generators = load_generators()
    usage = load_usage()
    footprints = load_footprints()

    models: dict[str, dict] = {}
    for name, classes in index.items():
        path = MODELS / f"{name}.vox"
        if not path.exists():
            continue
        parsed = parse_vox(path)
        stats = model_stats(parsed)
        per_class = {}
        for cls in classes:
            cpath = MODELS / f"{name}.{cls}.vox"
            if cpath.exists():
                per_class[cls] = model_stats(parse_vox(cpath))["count"]
        size = parsed["size"]
        models[name] = {
            "name": name,
            "classes": classes,
            "size": size,
            "tiles": [
                round(size[0] / TILE_VOXELS, 2),
                round(size[1] / TILE_VOXELS, 2),
                round(size[2] / TILE_VOXELS, 2),
            ],
            "voxels": stats["count"],
            "bbox": stats["bbox"],
            "class_voxels": per_class,
            "colors": [
                {
                    "index": idx,
                    "count": n,
                    "name": palette[idx - 1]["name"] if idx <= len(palette) else "?",
                    "rgb": palette[idx - 1]["rgb"] if idx <= len(palette) else "#ff00ff",
                    "class": palette[idx - 1]["class"] if idx <= len(palette) else "?",
                }
                for idx, n in sorted(stats["colors"].items(), key=lambda kv: -kv[1])
            ],
            "bytes": path.stat().st_size,
            "icon": f"{name}.png" if (ICONS / f"{name}.png").exists() else None,
            "preview": f"{name}.png" if (PREVIEWS / f"{name}.png").exists() else None,
            "source": generators.get(name),
            "used_by": usage.get(name, []),
        }

    assets = []
    for asset in group_models(list(models)):
        variations = [
            {"name": v.name, "role": v.role, "frame": v.frame}
            for v in sorted(
                asset.variations,
                key=lambda v: (
                    {"base": 0, "frame": 0, "fill": 1, "motion": 2}[v.role],
                    v.frame if v.frame is not None else -1,
                ),
            )
        ]
        classes = sorted({c for v in variations for c in models[v["name"]]["classes"]})
        primary = variations[0]["name"]
        src = models[primary]["source"] or {}
        assets.append(
            {
                "id": asset.id,
                "label": _label(asset.id),
                "category": asset.category,
                "variations": variations,
                "classes": classes,
                "icon": models[primary]["icon"],
                "companion": _companion(asset.id, set(models)),
                "doc": src.get("doc", ""),
                "note": src.get("note", ""),
                "voxels": sum(models[v["name"]]["voxels"] for v in variations),
                "footprint": next(
                    (
                        footprints[u["key"]]
                        for v in variations
                        for u in models[v["name"]]["used_by"]
                        if u["key"] in footprints
                    ),
                    None,
                ),
                "used_by": [u for v in variations for u in models[v["name"]]["used_by"]],
                "search": " ".join(
                    [asset.id, asset.category, " ".join(classes)]
                    + [v["name"] for v in variations]
                ).lower(),
            }
        )

    return {
        "repo": str(REPO),
        "tile_voxels": TILE_VOXELS,
        "materials": materials,
        "palette": palette,
        "categories": [c for c in CATEGORY_ORDER if any(a["category"] == c for a in assets)],
        "assets": assets,
        "models": models,
    }


def model_payload(name: str) -> dict:
    """Voxel geometry for one model, split the way Godot draws it."""
    index = json.loads((MODELS / "index.json").read_text())
    if name not in index:
        raise KeyError(name)
    combined = parse_vox(MODELS / f"{name}.vox")
    classes = {}
    for cls in index[name]:
        path = MODELS / f"{name}.{cls}.vox"
        if path.exists():
            p = parse_vox(path)
            classes[cls] = {"voxels": p["voxels"]}
    return {
        "name": name,
        "size": combined["size"],
        "palette": combined["palette"],
        "classes": classes,
    }
