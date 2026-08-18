<!--
Copyright (c) 2026 Jurjen Stellingwerff
SPDX-License-Identifier: LGPL-3.0-or-later
-->

# mesh3d — 3D geometry primitives for loft

```sh
loft install mesh3d
```

Aggregates three sub-modules in one package:

- `math` — `Vec2` / `Vec3` / `Vec4` + `Mat4` + vector / matrix ops
  (`add3` / `sub3` / `dot3` / `cross` / `length3` / `normalize3`;
  `mat4_identity` / `mat4_translate` / `mat4_scale` / `mat4_mul` /
  `mat4_transform` / `mat4_perspective` / `mat4_look_at` /
  `mat4_rotate_x` / `mat4_rotate_y` / `mat4_ortho` / `mat4_trs`).
- `mesh` — `Vertex` / `Triangle` / `Mesh` + `sphere()` builder +
  `mesh_to_floats` / `mesh_to_floats_uv` (flatten for GL upload).
- `scene` — `Scene` / `Node` / `Material` / `Camera` / `Light` +
  scene-graph constructors.

Consumers do `use mesh3d;` to get all the above types at the top level.

## The five contracts a signature does not carry

The geometry is right; the **bookkeeping** is what a caller gets wrong.  Every handle is a bare
`integer`, every buffer a bare `vector<single>`, every matrix a bare `vector<float>` — so each row
below is a well-typed call that produces a picture, just not the one that was asked for.  Each
links to a test that demonstrates the correct call and runs in CI, so it cannot go stale.

| contract | worked example |
|---|---|
| **A vertex index is a bare `integer`,** and `mesh_to_floats` does not fault on a bad one — it SKIPS that vertex, so the buffer is short by one *vertex*, no longer divides into triangles, and every float after the gap is read as part of the wrong one. | [`@MSH-001`](tests/worked-examples.loft) |
| **The stride is the contract.**  `mesh_to_floats` is 6 (pos+normal), `mesh_to_floats_uv` is 8 (pos+normal+uv), both `vector<single>`.  The buffer is sized by TRIANGLES — a shared vertex is written once per triangle — so its length says nothing about the vertex count, and a mesh with vertices but no triangles flattens to empty. | [`@MSH-002`](tests/worked-examples.loft) |
| **A face's direction is claimed twice** — once by the stored vertex normal, once by the winding of `add_quad` — and nothing checks they agree.  `plane` passes its corners reversed on purpose; the natural order points the face the other way while the normal still says up. | [`@MSH-003`](tests/worked-examples.loft) |
| **`mat4_mul(A, B)` applies B first,** and the layout is column-major, so a translation lives at `m[12..14]`, not `m[3]`/`m[7]`/`m[11]`.  Swapping the arguments is a well-typed call that puts the object somewhere else. | [`@MSH-004`](tests/worked-examples.loft) |
| **A degenerate input answers zero, never an error.**  `normalize3` of a zero vector is the zero vector, and every matrix builder guards its divisions with `?? 0.0` — so `mat4_look_at(eye, eye, up)` builds a matrix that collapses the whole world onto the origin. | [`@MSH-005`](tests/worked-examples.loft) |

## Why this package exists

These types previously lived as internal sub-modules inside the
`graphics` package.  Promoting them into a standalone package lets
non-rendering consumers (asset processors, the `glb` reader/writer,
headless physics, lavition's asset pipeline) use the types without
pulling in `graphics`'s OpenGL stack.

## Provenance

Extracted from the loft monorepo's `lib/graphics/src/{math,mesh,scene}.loft`
(via the registry-shipped graphics 0.1.0) on 2026-05-31 as part of
[@PLAN12 W.0b](https://github.com/jjstwerff/loft/blob/main/doc/claude/LAVITION.md).
