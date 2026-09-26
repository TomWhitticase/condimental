# Third-party notices

CONDIMENTAL itself is licensed under the GNU GPL v3 ([LICENSE](LICENSE)). It
is built on, and a distributed build of it contains, the third-party software
below. Every licence here is compatible with GPL-3.0; `deny.toml` holds the
allow-list and CI (`.github/workflows/audit.yml`, `cargo deny check licenses`)
refuses a dependency whose licence is not on it.

**A distributed build must carry these notices** — ship this file (or an
equivalent credits screen) with every exported game, alongside `LICENSE`.

## Godot Engine

The game runs on the [Godot Engine](https://godotengine.org/), which is
MIT-licensed: Copyright (c) 2014-present Godot Engine contributors, (c)
2007-2014 Juan Linietsky, Ariel Manzur. Godot bundles further third-party
libraries (FreeType, HarfBuzz, mbedTLS, zstd and others) under their own
permissive licences; their full texts are Godot's `COPYRIGHT.txt` and
`LICENSE.txt`, which a release build must include — Godot's
`Engine.get_license_text()` and `Engine.get_copyright_info()` return the same
information at runtime.

## MagicaVoxel Importer with Extensions (editor plugin)

Vendored in `godot_project/addons/MagicaVoxel_Importer_with_Extensions/`
(see its `VENDORED.md`). MIT License:

```
MIT License

Copyright (c) 2019 JohnCWakley

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Rust crates compiled into `sim_core`

The simulation library links these crates. Where a crate is dual-licensed,
CONDIMENTAL uses it under the MIT (or other permissive) option. Each crate's
full licence text is in its source distribution on
[crates.io](https://crates.io/).

| Crate | Version | Licence |
|---|---|---|
| az | 1.3.0 | MIT OR Apache-2.0 |
| bitflags | 2.13.1 | MIT OR Apache-2.0 |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT |
| cfg-if | 1.0.4 | MIT OR Apache-2.0 |
| fixed | 1.31.0 | MIT OR Apache-2.0 |
| foldhash | 0.2.0 | Zlib |
| glam | 0.32.1 | MIT OR Apache-2.0 |
| godot | 0.5.5 | MPL-2.0 |
| godot-cell | 0.5.5 | MPL-2.0 |
| godot-core | 0.5.5 | MPL-2.0 |
| godot-ffi | 0.5.5 | MPL-2.0 |
| godot-macros | 0.5.5 | MPL-2.0 |
| half | 2.7.1 | MIT OR Apache-2.0 |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 |
| hecs | 0.11.1 | MIT OR Apache-2.0 |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| libc | 0.2.189 | MIT OR Apache-2.0 |
| memchr | 2.8.3 | Unlicense OR MIT |
| once_cell | 1.21.4 | MIT OR Apache-2.0 |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| ron | 0.12.2 | MIT OR Apache-2.0 |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.151 | MIT OR Apache-2.0 |
| spin | 0.12.3 | MIT |
| syn | 2.0.119 | MIT OR Apache-2.0 |
| syn | 3.0.5 | MIT OR Apache-2.0 |
| typeid | 1.0.3 | MIT OR Apache-2.0 |
| typenum | 1.20.1 | MIT OR Apache-2.0 |
| unicode-ident | 1.0.24 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| venial | 0.6.1 | MIT |
| zerocopy | 0.8.56 | BSD-2-Clause OR Apache-2.0 OR MIT |
| zerocopy-derive | 0.8.56 | BSD-2-Clause OR Apache-2.0 OR MIT |
| zmij | 1.0.23 | MIT |

Regenerate that table after a dependency change with:

    cargo tree -p sim_core -e normal --prefix none --format '{p}|{l}'
