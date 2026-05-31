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
