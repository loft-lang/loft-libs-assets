<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# loft-libs-assets — file formats for loft

Multi-package chunk repo for **asset formats** — file readers and
writers that don't require a GPU (PNG, glTF binary, future audio /
font formats).  Each subdirectory is an independent loft package
published to the registry under its own name.

Per the chunked-repo design in
[loft's lib_plans/12-library-extraction/](https://github.com/jjstwerff/loft/blob/main/doc/claude/lib_plans/12-library-extraction/README.md)
§ Chunk grouping, and the 6-chunk topology in
[LAVITION.md](https://github.com/jjstwerff/loft/blob/main/doc/claude/LAVITION.md)
§ Library model.

## Why this chunk exists

Asset format libraries should be **headless-usable**: a Blender
export validator, an asset packer, lavition's editor asset
pipeline — none of these need OpenGL or audio devices.  Bundling
them with rendering (the previous `loft-libs-graphics` arrangement)
forced consumers to install a GPU stack to process a PNG.

## Packages

| Subdir | Package | Status |
|---|---|---|
| `glb/` | `glb` — glTF binary 3D scenes (meshes, materials, animations) | TODO (W.0b — promote from graphics submodule) |
| `imaging/` | `imaging` — PNG load / save + pixel manipulation | currently shipped in [`loft-libs-graphics`](https://github.com/loft-lang/loft-libs-graphics); migrates here at next major-version boundary |

## Installing a package

```sh
loft install glb           # 3D scene format
loft install imaging       # PNG (once migrated)
```

Consumers never see the chunk structure — they install per-package.

## Versioning + tags

Each package versions independently.  Git tags use the
**`<package>-v<version>`** convention to disambiguate sibling
packages in this multi-package repo (same as `loft-libs-core` and
`loft-libs-graphics`):

| Package + version | Git tag |
|---|---|
| glb 0.1.0 | `glb-v0.1.0` |
| imaging 0.1.0 (when migrated) | `imaging-v0.1.0` |

## License

LGPL-3.0-or-later — see [LICENSE](LICENSE).
