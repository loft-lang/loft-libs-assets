<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# loft-libs-assets — file formats for loft

Multi-package chunk repo for **asset formats** — file readers and
writers that don't require a GPU (content packs, glTF binary, future audio /
font formats).  Each subdirectory is an independent loft package
published to the registry under its own name.

Per the chunked-repo design in
[loft's lib_plans/12-library-extraction/](https://github.com/loft-lang/loft/blob/main/doc/claude/lib_plans/12-library-extraction/README.md)
§ Chunk grouping, and the chunk topology in
[LAVITION.md](https://github.com/loft-lang/loft/blob/main/doc/claude/LAVITION.md)
§ Library model.

## Why this chunk exists

Asset format libraries should be **headless-usable**: a Blender
export validator, an asset packer, lavition's editor asset
pipeline — none of these need OpenGL or audio devices.  Bundling
them with rendering (the previous `loft-libs-graphics` arrangement)
forced consumers to install a GPU stack to process a PNG.

## Packages

| Subdir | Package |
|---|---|
| [`assets/`](assets/) | `assets` — a game content pack that IS a loft store (art, audio, fonts, scenes; range-readable) |
| [`mesh3d/`](mesh3d/) | `mesh3d` — 3D geometry primitives (Vec/Mat, Mesh, Scene) |
| [`glb/`](glb/) | `glb` — glTF 2.0 binary writer (depends on mesh3d) |

Each package's version is the `version` in its `loft.toml`; the registry lists every
published one (`loft api --registry`).  Every package has a guide at
`docs/01-getting-started.loft`.  PNG reading and writing (`imaging`) lives in
[`loft-libs-graphics`](https://github.com/loft-lang/loft-libs-graphics).

## Installing a package

```sh
loft install assets        # a content pack read from disk or by HTTP range
loft install mesh3d        # 3D geometry types (Vec, Mat, Mesh, Scene)
loft install glb           # glTF 2.0 binary writer (pulls mesh3d transitively)
```

Consumers never see the chunk structure — they install per-package.

## Versioning + tags

Each package versions independently.  Git tags use the
**`<package>-v<version>`** convention to disambiguate sibling
packages in this multi-package repo (`glb-v0.1.4`), as `loft-libs-core` and
`loft-libs-graphics` do.  They are published by loft's maintainers through the
signed registry flow —
[LIBRARY_PUBLISH.md](https://github.com/loft-lang/loft/blob/main/doc/claude/LIBRARY_PUBLISH.md).

## License

LGPL-3.0-or-later — see [LICENSE](LICENSE).
