// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// glb-reference — the pure-Rust twin of the `glb` package's performance pass
// (bench/bench.loft), one file built with `rustc -O`.  It builds the SAME meshes and scenes
// with the SAME arithmetic, writes the SAME .glb bytes, and prints the same rows, hash
// included: a row whose hash matches the loft build's is a like-for-like comparison, and
// only then is a routine's loft time judged against it (@FR-Perf-Weight).  No dependencies
// and no cleverness — plain idiomatic Rust, the speed an industry implementation reaches
// without effort, which is exactly what the bar should be.
//
//     rustc -O --edition=2021 bench/bench.rs -o bench/.build/stats_rs && bench/.build/stats_rs --n 2
//
// Run it from the package directory; it writes its scratch files under `bench/.build/`.
// Each routine is a port of its loft original (glb/src/glb.loft over mesh3d's types): the
// JSON is `write!` into one String, the binary chunk `extend_from_slice` into one Vec<u8>,
// and the file is written once.  `black_box` guards each op's INPUT and the sink — never
// anything inside a kernel.
use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const SEG: i64 = 128;
const NODES: i64 = 1000;
const MATERIALS: i64 = 20;
const LIGHTS: i64 = 10;

fn fnv_bytes(h0: i64, v: &[u8]) -> i64 {
    let mut h = h0;
    for &b in v {
        h = ((h ^ b as i64) * FNV_PRIME) & 0xFFFF_FFFF;
    }
    h
}

struct Row {
    name: &'static str,
    iters: i64,
    us: i64,
    px: i64,
    hash: i64,
    sink: i64,
}

fn print_row(r: &Row) {
    let ns_op = r.us * 1000 / r.iters;
    let ns_px = if r.px > 0 { (r.us * 1000) as f64 / (r.iters * r.px) as f64 } else { 0.0 };
    println!("{}\t{}\t{}\t{}\t{}\t{:.3}\t{:x}", r.name, r.iters, r.us, ns_op, r.px, ns_px, r.hash);
}

// ── mesh3d: math.loft, mesh.loft, scene.loft ────────────────────────

#[derive(Clone, Copy)]
struct Vec2 {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy)]
struct Vec3 {
    x: f64,
    y: f64,
    z: f64,
}

fn vec3(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3 { x, y, z }
}

fn normalize3(v: Vec3) -> Vec3 {
    let l = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    if l == 0.0 {
        return vec3(0.0, 0.0, 0.0);
    }
    vec3(v.x / l, v.y / l, v.z / l)
}

#[derive(Clone, Copy)]
struct Mat4 {
    m: [f64; 16],
}

fn mat4_identity() -> Mat4 {
    Mat4 { m: [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0] }
}

#[allow(clippy::too_many_arguments)]
fn mat4_trs(tx: f64, ty: f64, tz: f64, ry: f64, sx: f64, sy: f64, sz: f64) -> Mat4 {
    let (c, s) = (ry.cos(), ry.sin());
    Mat4 { m: [c * sx, 0.0, s * sx, 0.0, 0.0, sy, 0.0, 0.0, -s * sz, 0.0, c * sz, 0.0, tx, ty, tz, 1.0] }
}

#[derive(Clone, Copy)]
struct Vertex {
    pos: Vec3,
    normal: Vec3,
    uv: Vec2,
}

struct Mesh {
    name: String,
    vertices: Vec<Vertex>,
    triangles: Vec<[u32; 3]>,
}

impl Mesh {
    fn new(name: &str) -> Mesh {
        Mesh { name: name.to_string(), vertices: Vec::new(), triangles: Vec::new() }
    }
    fn add_vertex(&mut self, pos: Vec3, normal: Vec3, uv: (f64, f64)) -> u32 {
        let idx = self.vertices.len() as u32;
        self.vertices.push(Vertex { pos, normal, uv: Vec2 { x: uv.0, y: uv.1 } });
        idx
    }
    fn add_quad(&mut self, i0: u32, i1: u32, i2: u32, i3: u32) {
        self.triangles.push([i0, i1, i2]);
        self.triangles.push([i0, i2, i3]);
    }
}

fn cube() -> Mesh {
    let mut m = Mesh::new("cube");
    // (normal, four corners) per face, in mesh.loft's order.
    let faces: [([f64; 3], [[f64; 3]; 4]); 6] = [
        ([0.0, 0.0, 1.0], [[-0.5, -0.5, 0.5], [0.5, -0.5, 0.5], [0.5, 0.5, 0.5], [-0.5, 0.5, 0.5]]),
        ([0.0, 0.0, -1.0], [[0.5, -0.5, -0.5], [-0.5, -0.5, -0.5], [-0.5, 0.5, -0.5], [0.5, 0.5, -0.5]]),
        ([1.0, 0.0, 0.0], [[0.5, -0.5, 0.5], [0.5, -0.5, -0.5], [0.5, 0.5, -0.5], [0.5, 0.5, 0.5]]),
        ([-1.0, 0.0, 0.0], [[-0.5, -0.5, -0.5], [-0.5, -0.5, 0.5], [-0.5, 0.5, 0.5], [-0.5, 0.5, -0.5]]),
        ([0.0, 1.0, 0.0], [[-0.5, 0.5, 0.5], [0.5, 0.5, 0.5], [0.5, 0.5, -0.5], [-0.5, 0.5, -0.5]]),
        ([0.0, -1.0, 0.0], [[-0.5, -0.5, -0.5], [0.5, -0.5, -0.5], [0.5, -0.5, 0.5], [-0.5, -0.5, 0.5]]),
    ];
    let uvs = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
    for (n, corners) in faces.iter() {
        let nv = vec3(n[0], n[1], n[2]);
        let mut idx = [0u32; 4];
        for k in 0..4 {
            let c = corners[k];
            idx[k] = m.add_vertex(vec3(c[0], c[1], c[2]), nv, uvs[k]);
        }
        m.add_quad(idx[0], idx[1], idx[2], idx[3]);
    }
    m
}

fn plane(name: &str, pw: f64, pd: f64) -> Mesh {
    let mut m = Mesh::new(name);
    let hw = pw / 2.0;
    let hd = pd / 2.0;
    let n = vec3(0.0, 1.0, 0.0);
    let v0 = m.add_vertex(vec3(-hw, 0.0, -hd), n, (0.0, 0.0));
    let v1 = m.add_vertex(vec3(hw, 0.0, -hd), n, (1.0, 0.0));
    let v2 = m.add_vertex(vec3(hw, 0.0, hd), n, (1.0, 1.0));
    let v3 = m.add_vertex(vec3(-hw, 0.0, hd), n, (0.0, 1.0));
    m.add_quad(v0, v3, v2, v1);
    m
}

fn sphere(name: &str, sr: f64, ss: i64, st: i64) -> Mesh {
    let mut sm = Mesh::new(name);
    let pi2 = 6.283185307179586;
    for row in 0..st + 1 {
        let phi = 3.141592653589793 * (row as f64) / (st as f64);
        let sy = sr * phi.cos();
        let ring_r = sr * phi.sin();
        for col in 0..ss + 1 {
            let theta = pi2 * (col as f64) / (ss as f64);
            let pos = vec3(ring_r * theta.cos(), sy, ring_r * theta.sin());
            sm.add_vertex(pos, normalize3(pos), ((col as f64) / (ss as f64), (row as f64) / (st as f64)));
        }
    }
    for row in 0..st {
        for col in 0..ss {
            let i00 = (row * (ss + 1) + col) as u32;
            let i01 = (row * (ss + 1) + col + 1) as u32;
            let i10 = ((row + 1) * (ss + 1) + col) as u32;
            let i11 = ((row + 1) * (ss + 1) + col + 1) as u32;
            sm.add_quad(i00, i01, i11, i10);
        }
    }
    sm
}

struct Material {
    name: String,
    r: f64,
    g: f64,
    b: f64,
    a: f64,
    metallic: f64,
    roughness: f64,
}

fn material_color(name: String, r: f64, g: f64, b: f64) -> Material {
    Material { name, r, g, b, a: 1.0, metallic: 0.0, roughness: 0.5 }
}

fn material_metal(name: String, r: f64, g: f64, b: f64, rough: f64) -> Material {
    Material { name, r, g, b, a: 1.0, metallic: 1.0, roughness: rough }
}

struct Node {
    name: String,
    mesh_idx: i64,
    material_idx: i64,
    transform: Mat4,
}

#[derive(PartialEq)]
enum LightType {
    Directional,
    Point,
}

struct Light {
    name: String,
    light_type: LightType,
    r: f64,
    g: f64,
    b: f64,
    intensity: f64,
    position: Vec3,
    direction: Vec3,
}

struct Scene {
    name: String,
    meshes: Vec<Mesh>,
    materials: Vec<Material>,
    nodes: Vec<Node>,
    lights: Vec<Light>,
}

// ── glb.loft ────────────────────────────────────────────────────────

fn pad4(n: usize) -> usize {
    let r = n % 4;
    if r == 0 { 0 } else { 4 - r }
}

fn pos_min_max(verts: &[Vertex]) -> (Vec3, Vec3) {
    if verts.is_empty() {
        return (vec3(0.0, 0.0, 0.0), vec3(0.0, 0.0, 0.0));
    }
    let mut lo = verts[0].pos;
    let mut hi = verts[0].pos;
    for v in verts {
        if v.pos.x < lo.x { lo.x = v.pos.x; }
        if v.pos.y < lo.y { lo.y = v.pos.y; }
        if v.pos.z < lo.z { lo.z = v.pos.z; }
    }
    for v in verts {
        if v.pos.x > hi.x { hi.x = v.pos.x; }
        if v.pos.y > hi.y { hi.y = v.pos.y; }
        if v.pos.z > hi.z { hi.z = v.pos.z; }
    }
    (lo, hi)
}

/// POSITION, NORMAL, TEXCOORD_0 and the indices of one mesh, appended little-endian.
fn put_mesh_bin(out: &mut Vec<u8>, m: &Mesh) {
    for v in &m.vertices {
        for f in [v.pos.x, v.pos.y, v.pos.z] { out.extend_from_slice(&(f as f32).to_le_bytes()); }
    }
    for v in &m.vertices {
        for f in [v.normal.x, v.normal.y, v.normal.z] { out.extend_from_slice(&(f as f32).to_le_bytes()); }
    }
    for v in &m.vertices {
        for f in [v.uv.x, v.uv.y] { out.extend_from_slice(&(f as f32).to_le_bytes()); }
    }
    for t in &m.triangles {
        for &i in t { out.extend_from_slice(&(i as i32).to_le_bytes()); }
    }
}

fn mesh_bin_bytes(m: &Mesh) -> usize {
    let nv = m.vertices.len();
    nv * 12 + nv * 12 + nv * 8 + m.triangles.len() * 3 * 4
}

/// The GLB container around a JSON header and a binary chunk.
fn write_glb(path: &str, json: &str, bin_len: usize, put_bin: impl FnOnce(&mut Vec<u8>)) {
    let json_pad = pad4(json.len());
    let json_chunk = json.len() + json_pad;
    let total = 12 + 8 + json_chunk + 8 + bin_len;
    let mut out: Vec<u8> = Vec::with_capacity(total);
    for w in [0x46546C67u32, 2, total as u32, json_chunk as u32, 0x4E4F534A] {
        out.extend_from_slice(&w.to_le_bytes());
    }
    out.extend_from_slice(json.as_bytes());
    out.resize(out.len() + json_pad, b' ');
    for w in [bin_len as u32, 0x004E4942] {
        out.extend_from_slice(&w.to_le_bytes());
    }
    put_bin(&mut out);
    std::fs::write(path, &out).expect("write glb");
}

fn glb_json(m: &Mesh, lo: Vec3, hi: Vec3) -> String {
    let nv = m.vertices.len();
    let ni = m.triangles.len() * 3;
    let (pos_bytes, norm_bytes, uv_bytes, idx_bytes) = (nv * 12, nv * 12, nv * 8, ni * 4);
    let bin_bytes = pos_bytes + norm_bytes + uv_bytes + idx_bytes;
    let off_norm = pos_bytes;
    let off_uv = pos_bytes + norm_bytes;
    let off_idx = pos_bytes + norm_bytes + uv_bytes;
    let mut s = String::new();
    s.push_str("{\"asset\":{\"version\":\"2.0\",\"generator\":\"loft\"},");
    s.push_str("\"scene\":0,\"scenes\":[{\"nodes\":[0]}],\"nodes\":[{\"mesh\":0}],");
    let _ = write!(s, "\"meshes\":[{{\"name\":\"{}\",\"primitives\":[{{\"attributes\":{{\"POSITION\":0,\"NORMAL\":1,\"TEXCOORD_0\":2}},\"indices\":3}}]}}],", m.name);
    s.push_str("\"accessors\":[");
    let _ = write!(s, "{{\"bufferView\":0,\"componentType\":5126,\"count\":{nv},\"type\":\"VEC3\",\"min\":[{:.6},{:.6},{:.6}],\"max\":[{:.6},{:.6},{:.6}]}},", lo.x, lo.y, lo.z, hi.x, hi.y, hi.z);
    let _ = write!(s, "{{\"bufferView\":1,\"componentType\":5126,\"count\":{nv},\"type\":\"VEC3\"}},");
    let _ = write!(s, "{{\"bufferView\":2,\"componentType\":5126,\"count\":{nv},\"type\":\"VEC2\"}},");
    let _ = write!(s, "{{\"bufferView\":3,\"componentType\":5125,\"count\":{ni},\"type\":\"SCALAR\"}}");
    s.push_str("],");
    s.push_str("\"bufferViews\":[");
    let _ = write!(s, "{{\"buffer\":0,\"byteOffset\":0,\"byteLength\":{pos_bytes},\"target\":34962}},");
    let _ = write!(s, "{{\"buffer\":0,\"byteOffset\":{off_norm},\"byteLength\":{norm_bytes},\"target\":34962}},");
    let _ = write!(s, "{{\"buffer\":0,\"byteOffset\":{off_uv},\"byteLength\":{uv_bytes},\"target\":34962}},");
    let _ = write!(s, "{{\"buffer\":0,\"byteOffset\":{off_idx},\"byteLength\":{idx_bytes},\"target\":34963}}");
    s.push_str("],");
    let _ = write!(s, "\"buffers\":[{{\"byteLength\":{bin_bytes}}}]}}");
    s
}

fn save_glb(m: &Mesh, path: &str) {
    let (lo, hi) = pos_min_max(&m.vertices);
    let json = glb_json(m, lo, hi);
    write_glb(path, &json, mesh_bin_bytes(m), |out| put_mesh_bin(out, m));
}

fn material_json(s: &mut String, mat: &Material) {
    let (cr, cg, cb, ca) = (mat.r as f32, mat.g as f32, mat.b as f32, mat.a as f32);
    let (me, ro) = (mat.metallic as f32, mat.roughness as f32);
    let _ = write!(s, "{{\"name\":\"{}\",", mat.name);
    s.push_str("\"pbrMetallicRoughness\":{");
    let _ = write!(s, "\"baseColorFactor\":[{cr:.6},{cg:.6},{cb:.6},{ca:.6}],");
    let _ = write!(s, "\"metallicFactor\":{me:.6},");
    let _ = write!(s, "\"roughnessFactor\":{ro:.6}");
    s.push_str("}}");
}

fn is_identity(t: &Mat4) -> bool {
    t.m == mat4_identity().m
}

fn mat4_json(s: &mut String, t: &Mat4) {
    s.push_str("\"matrix\":[");
    for (i, v) in t.m.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        let _ = write!(s, "{:.6}", *v as f32);
    }
    s.push(']');
}

fn light_node_json(s: &mut String, lt: &Light, li: usize) {
    let _ = write!(s, "{{\"name\":\"{}\",\"extensions\":{{\"KHR_lights_punctual\":{{\"light\":{li}}}}}", lt.name);
    if lt.light_type == LightType::Point {
        let (px, py, pz) = (lt.position.x as f32, lt.position.y as f32, lt.position.z as f32);
        let _ = write!(s, ",\"translation\":[{px:.6},{py:.6},{pz:.6}]");
    } else {
        let dir = normalize3(lt.direction);
        let (dx, dy, dz) = (dir.x, dir.y, dir.z);
        let dot = -dz;
        let (qx, qy, qz, qw): (f32, f32, f32, f32) = if dot < -0.9999 {
            (0.0, 1.0, 0.0, 0.0)
        } else if dot > 0.9999 {
            (0.0, 0.0, 0.0, 1.0)
        } else {
            let half = 1.0 + dot;
            let (ax, ay, az) = (dy, -dx, 0.0);
            let root = (ax * ax + ay * ay + az * az + half * half).sqrt();
            let inv = if root == 0.0 { 0.0 } else { 1.0 / root };
            ((ax * inv) as f32, (ay * inv) as f32, (az * inv) as f32, (half * inv) as f32)
        };
        let _ = write!(s, ",\"rotation\":[{qx:.6},{qy:.6},{qz:.6},{qw:.6}]");
    }
    s.push('}');
}

fn scene_json(sc: &Scene, bin_offset: &[usize]) -> String {
    let mesh_mat: Vec<i64> = (0..sc.meshes.len())
        .map(|mi| {
            let mut found = 0;
            for nd in &sc.nodes {
                if nd.mesh_idx == mi as i64 {
                    found = nd.material_idx;
                }
            }
            found
        })
        .collect();
    let n_lights = sc.lights.len();
    let total_nodes = sc.nodes.len() + n_lights;
    let mut s = String::new();
    s.push_str("{\"asset\":{\"version\":\"2.0\",\"generator\":\"loft\"},");
    if n_lights > 0 {
        s.push_str("\"extensionsUsed\":[\"KHR_lights_punctual\"],");
        s.push_str("\"extensionsRequired\":[\"KHR_lights_punctual\"],");
        s.push_str("\"extensions\":{\"KHR_lights_punctual\":{\"lights\":[");
        for (li, lt) in sc.lights.iter().enumerate() {
            if li > 0 {
                s.push(',');
            }
            let (cr, cg, cb, inten) = (lt.r as f32, lt.g as f32, lt.b as f32, lt.intensity as f32);
            let kind = if lt.light_type == LightType::Point { "point" } else { "directional" };
            let _ = write!(s, "{{\"name\":\"{}\",\"type\":\"{kind}\",\"color\":[{cr:.6},{cg:.6},{cb:.6}],\"intensity\":{inten:.6}}}", lt.name);
        }
        s.push_str("]}},");
    }
    let _ = write!(s, "\"scene\":0,\"scenes\":[{{\"name\":\"{}\",\"nodes\":[", sc.name);
    for ni in 0..total_nodes {
        if ni > 0 {
            s.push(',');
        }
        let _ = write!(s, "{ni}");
    }
    s.push_str("]}],");
    s.push_str("\"nodes\":[");
    let mut ni = 0;
    for nd in &sc.nodes {
        if ni > 0 {
            s.push(',');
        }
        if is_identity(&nd.transform) {
            let _ = write!(s, "{{\"name\":\"{}\",\"mesh\":{}}}", nd.name, nd.mesh_idx);
        } else {
            let _ = write!(s, "{{\"name\":\"{}\",\"mesh\":{},", nd.name, nd.mesh_idx);
            mat4_json(&mut s, &nd.transform);
            s.push('}');
        }
        ni += 1;
    }
    for (li, lt) in sc.lights.iter().enumerate() {
        if ni > 0 {
            s.push(',');
        }
        light_node_json(&mut s, lt, li);
        ni += 1;
    }
    s.push_str("],");
    s.push_str("\"meshes\":[");
    for (mi, me) in sc.meshes.iter().enumerate() {
        if mi > 0 {
            s.push(',');
        }
        let _ = write!(s, "{{\"name\":\"{}\",\"primitives\":[{{\"attributes\":{{\"POSITION\":{},\"NORMAL\":{},\"TEXCOORD_0\":{}}},\"indices\":{},\"material\":{}}}]}}",
                       me.name, mi * 4, mi * 4 + 1, mi * 4 + 2, mi * 4 + 3, mesh_mat[mi]);
    }
    s.push_str("],");
    s.push_str("\"accessors\":[");
    for (mi, me) in sc.meshes.iter().enumerate() {
        if mi > 0 {
            s.push(',');
        }
        let nv = me.vertices.len();
        let nidx = me.triangles.len() * 3;
        let (lo, hi) = pos_min_max(&me.vertices);
        let (minx, miny, minz) = (lo.x as f32, lo.y as f32, lo.z as f32);
        let (maxx, maxy, maxz) = (hi.x as f32, hi.y as f32, hi.z as f32);
        let _ = write!(s, "{{\"bufferView\":{},\"componentType\":5126,\"count\":{nv},\"type\":\"VEC3\",\"min\":[{minx:.6},{miny:.6},{minz:.6}],\"max\":[{maxx:.6},{maxy:.6},{maxz:.6}]}},", mi * 4);
        let _ = write!(s, "{{\"bufferView\":{},\"componentType\":5126,\"count\":{nv},\"type\":\"VEC3\"}},", mi * 4 + 1);
        let _ = write!(s, "{{\"bufferView\":{},\"componentType\":5126,\"count\":{nv},\"type\":\"VEC2\"}},", mi * 4 + 2);
        let _ = write!(s, "{{\"bufferView\":{},\"componentType\":5125,\"count\":{nidx},\"type\":\"SCALAR\"}}", mi * 4 + 3);
    }
    s.push_str("],");
    s.push_str("\"bufferViews\":[");
    for (mi, me) in sc.meshes.iter().enumerate() {
        if mi > 0 {
            s.push(',');
        }
        let nv = me.vertices.len();
        let nidx = me.triangles.len() * 3;
        let (pos_bytes, norm_bytes, uv_bytes, idx_bytes) = (nv * 12, nv * 12, nv * 8, nidx * 4);
        let base = bin_offset[mi];
        let off_norm = base + pos_bytes;
        let off_uv = base + pos_bytes + norm_bytes;
        let off_idx = base + pos_bytes + norm_bytes + uv_bytes;
        let _ = write!(s, "{{\"buffer\":0,\"byteOffset\":{base},\"byteLength\":{pos_bytes},\"target\":34962}},");
        let _ = write!(s, "{{\"buffer\":0,\"byteOffset\":{off_norm},\"byteLength\":{norm_bytes},\"target\":34962}},");
        let _ = write!(s, "{{\"buffer\":0,\"byteOffset\":{off_uv},\"byteLength\":{uv_bytes},\"target\":34962}},");
        let _ = write!(s, "{{\"buffer\":0,\"byteOffset\":{off_idx},\"byteLength\":{idx_bytes},\"target\":34963}}");
    }
    s.push_str("],");
    s.push_str("\"materials\":[");
    for (mi, mat) in sc.materials.iter().enumerate() {
        if mi > 0 {
            s.push(',');
        }
        material_json(&mut s, mat);
    }
    s.push_str("],");
    let total_bin: usize = sc.meshes.iter().map(mesh_bin_bytes).sum();
    let _ = write!(s, "\"buffers\":[{{\"byteLength\":{total_bin}}}]}}");
    s
}

fn save_scene_glb(sc: &Scene, path: &str) {
    let mut bin_offset = Vec::with_capacity(sc.meshes.len());
    let mut cur = 0;
    for me in &sc.meshes {
        bin_offset.push(cur);
        cur += mesh_bin_bytes(me);
    }
    let json = scene_json(sc, &bin_offset);
    write_glb(path, &json, cur, |out| {
        for me in &sc.meshes {
            put_mesh_bin(out, me);
        }
    });
}

// ── The rows ────────────────────────────────────────────────────────

fn scratch(what: &str, stamp: u128) -> String {
    format!("bench/.build/{what}_rs_{stamp}.glb")
}

fn hash_file(path: &str) -> i64 {
    let bytes = std::fs::read(path).unwrap_or_default();
    let _ = std::fs::remove_file(path);
    fnv_bytes(FNV_OFFSET, &bytes)
}

fn file_size(path: &str) -> i64 {
    std::fs::metadata(path).map(|m| m.len() as i64).unwrap_or(0)
}

fn bench_save(n: i64, stamp: u128) -> Row {
    let a = sphere("even", 1.0, SEG, SEG);
    let b = sphere("odd", 1.5, SEG, SEG);
    let path = scratch("save", stamp);
    let t0 = Instant::now();
    let mut sink = 0i64;
    for r in 0..n {
        let r = black_box(r);
        save_glb(black_box(if (r & 1) == 0 { &a } else { &b }), &path);
        sink += file_size(&path);
    }
    let us = t0.elapsed().as_micros() as i64;
    save_glb(black_box(&a), &path);
    Row { name: "save_glb", iters: n, us, px: a.vertices.len() as i64, hash: hash_file(&path), sink: black_box(sink) }
}

fn build_scene(k: f64) -> Scene {
    let mut sc = Scene { name: "hall".to_string(), meshes: Vec::new(), materials: Vec::new(),
                         nodes: Vec::new(), lights: Vec::new() };
    sc.meshes.push(cube());
    sc.meshes.push(plane("floor", 8.0 * k, 6.0));
    sc.meshes.push(sphere("orb", 0.5 * k, 8, 6));
    for mi in 0..MATERIALS {
        let mf = (mi as f64) / (MATERIALS as f64);
        sc.materials.push(if (mi & 1) == 0 {
            material_color(format!("paint_{mi}"), mf * k, 0.5, 1.0 - mf)
        } else {
            material_metal(format!("metal_{mi}"), 0.9, mf, 0.25 * k, 0.1 + mf * 0.5)
        });
    }
    for ni in 0..NODES {
        let nf = ni as f64;
        sc.nodes.push(if ni % 10 == 0 {
            Node { name: format!("still_{ni}"), mesh_idx: ni % 3, material_idx: ni % MATERIALS,
                   transform: mat4_identity() }
        } else {
            Node { name: format!("node_{ni}"), mesh_idx: ni % 3, material_idx: ni % MATERIALS,
                   transform: mat4_trs(nf * 0.5 * k, (ni % 7) as f64, -nf * 0.25, nf * 0.1,
                                       k, 1.0 + (ni % 4) as f64 * 0.25, 2.0 - k) }
        });
    }
    for li in 0..LIGHTS {
        let lf = li as f64;
        let light = |name: String, kind: LightType, r: f64, g: f64, b: f64, i: f64, p: Vec3, d: Vec3| {
            Light { name, light_type: kind, r, g, b, intensity: i, position: p, direction: d }
        };
        sc.lights.push(if (li & 1) == 0 {
            light(format!("lamp_{li}"), LightType::Point, 1.0, 0.8 * k, 0.6, 10.0 + lf,
                  vec3(lf, 3.0 * k, -lf), vec3(0.0, -1.0, 0.0))
        } else if li == 1 {
            light(format!("sun_{li}"), LightType::Directional, 1.0, 1.0, 0.9, 3.0 * k,
                  vec3(0.0, 0.0, 0.0), vec3(0.0, 0.0, -1.0))
        } else if li == 3 {
            light(format!("sun_{li}"), LightType::Directional, 1.0, 1.0, 0.9, 3.0 * k,
                  vec3(0.0, 0.0, 0.0), vec3(0.0, 0.0, 1.0))
        } else {
            light(format!("sun_{li}"), LightType::Directional, 0.9, 1.0, 1.0, 2.0,
                  vec3(0.0, 0.0, 0.0), vec3(lf * 0.1 * k, -1.0, 0.3))
        });
    }
    sc
}

fn bench_scene(n: i64, stamp: u128) -> Row {
    let a = build_scene(1.0);
    let b = build_scene(1.25);
    let path = scratch("scene", stamp);
    let t0 = Instant::now();
    let mut sink = 0i64;
    for r in 0..n {
        let r = black_box(r);
        save_scene_glb(black_box(if (r & 1) == 0 { &a } else { &b }), &path);
        sink += file_size(&path);
    }
    let us = t0.elapsed().as_micros() as i64;
    save_scene_glb(black_box(&a), &path);
    Row { name: "save_scene_glb", iters: n, us, px: NODES + MATERIALS + LIGHTS, hash: hash_file(&path),
          sink: black_box(sink) }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut n: i64 = 20;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--n" && i + 1 < args.len() {
            n = args[i + 1].parse().unwrap_or(20);
            i += 1;
        }
        i += 1;
    }
    if n < 1 {
        n = 1;
    }
    let _ = std::fs::create_dir_all("bench/.build");
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let t0 = Instant::now();
    println!("routine\titers\tus\tns_op\tpx\tns_px\thash");
    let rows = [bench_save(n, stamp), bench_scene(n, stamp)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
