<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# glb — glTF 2.0 binary (.glb) writer for loft

```sh
loft install glb
```

Exports a `Mesh` or `Scene` (from `mesh3d`) as a GLB 2.0 file
readable by Blender, three.js, gltf-validator, and any other
glTF 2.0 consumer.

## Surface

- `save_glb(m: mesh::Mesh, path: text)` — write a single mesh as a
  GLB scene with one node + one mesh.
- `save_scene_glb(sc: scene::Scene, path: text)` — write a full
  scene (multiple meshes, materials, lights, camera nodes,
  transforms) as a GLB.

Output format: glTF 2.0 binary spec, JSON chunk + BIN chunk, RGB
positions + normals + UVs, indexed triangles, pbrMetallicRoughness
materials, directional + point lights via the glTF light extension
convention.

## Dependencies

- [`mesh3d`](https://github.com/loft-lang/loft-libs-assets/tree/main/mesh3d)
  — `Vertex` / `Mesh` / `Scene` / `Mat4` / `Vec3` types this writer
  serialises.

## Headless usable

Pure-loft package — no native code, no OpenGL, no GPU.  Useful
for:

- Asset-processing pipelines (validate / convert / repack glb
  files without instantiating a renderer)
- Server-side mesh generation (multiplayer levels authored
  procedurally and shipped to clients)
- lavition's editor asset pipeline (import / inspect / export glb
  without holding a viewport open)

## Provenance

Extracted from the loft monorepo's `lib/graphics/src/glb.loft`
(via the registry-shipped graphics 0.1.0) on 2026-05-31 as part of
[@PLAN12 W.0b](https://github.com/jjstwerff/loft/blob/main/doc/claude/LAVITION.md).
Previously a submodule inside `graphics`; promoted to a standalone
package so non-rendering consumers don't need to install OpenGL.
