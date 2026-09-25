# The Sauce Book — archive

Source and art for the 2026-09-25 design doc *The Sauce Book — nine sauces after
BBQ (CONDIMENTAL)*:
<https://docs.google.com/document/d/1w8sh74PZ1yvRpCScILd6rBnswa6Ed-Qmx2ULOx0YRsk/edit>

This branch exists so the doc's assets outlive the session scratchpad they were
made in. It is a reference archive, not a proposal to merge.

The art is **concept art**: a small Python isometric voxel renderer that reads
the game's real palette (`assets/voxels/palette.ron`) and shipped icons, so it
sits in the game's colour world. It is not the game's voxel pipeline — a sauce
that ships still needs its models built through `assets/voxels/`.

## Layout

```
art/
  voxlib.py            tiny isometric voxel renderer + palette loader
  models.py            ingredient / product voxel models
  machines.py          proposed machine models
  cards.py, plate.py   recipe-card and full-page plate layout
  build_cards.py       → card_*.jpg   (one per sauce)
  build_plates.py      → plate_*.jpg  (cover, sauce tree, raws, machines, variants)
  scratch/             check renders and contact sheets from iteration
doc/
  body_1..4.html       the doc text, in four parts
  build_html.py        → sauce_book.html (every image inlined as a data URI)
  sauce_book.html      the whole doc, self-contained — open it in a browser
  sethtml.swift        puts an HTML file on the macOS pasteboard as rich HTML
```

## Rebuilding

Needs Python 3 with Pillow. From this directory:

```sh
cd art && python3 build_cards.py && python3 build_plates.py
cd ../doc && python3 build_html.py
```

To re-paste into a Google Doc: `swiftc sethtml.swift -o sethtml &&
./sethtml sauce_book.html`, then paste into the doc body in Chrome.

`sauce_book.html` here was rebuilt at 17:10, after the Google Doc's last edit
(16:11). If the two differ, this file is the later version.
