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

## The four contracts a signature does not carry

The whole public surface is two functions that return **nothing** — `save_glb(mesh, path)` and
`save_scene_glb(scene, path)` are statements, with no success value, no error and no null.  The
only evidence a save happened is the file, and the writer will happily produce a well-formed GLB
container around a glTF no reader will accept.  Each row links to a test that runs in CI.

| contract | worked example |
|---|---|
| **The save answers nothing,** so the file is the only evidence: check it exists, is non-empty, and carries the `glTF` magic whose total-length field matches the real size.  A save truncates first, so writing one path twice replaces rather than appends. | [`@GLB-001`](tests/worked-examples.loft) |
| **`save_glb` writes one mesh and nothing else** — no material, no node transform, no camera, no light.  `save_scene_glb` is the door that carries them.  (Node transforms are compared to the identity by *exact* float equality, so a full-turn rotation still emits a `matrix`.) | [`@GLB-002`](tests/worked-examples.loft) |
| **A triangle index is copied into the file unchecked.**  The same out-of-range index that `mesh3d`'s `mesh_to_floats` silently skips is written verbatim here, so one defect fails two ways: a sheared GL buffer, or a structurally perfect GLB whose index 5 stands against an accessor declaring 3 vertices. | [`@GLB-003`](tests/worked-examples.loft) |
| **An empty mesh writes a valid container around an invalid glTF** — right magic, right chunk headers, matching total length, and accessors declaring `"count":0`, which glTF 2.0 forbids.  Validate before saving; nothing downstream will. | [`@GLB-004`](tests/worked-examples.loft) |

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
