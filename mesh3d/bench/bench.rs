// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// mesh3d-reference — the pure-Rust twin of the `mesh3d` package's performance pass
// (bench/bench.loft), one file built with `rustc -O`.  It computes the SAME workloads with
// the SAME arithmetic, in the same order, and prints the same rows, hash included: a row
// whose hash matches the loft build's is a like-for-like comparison, and only then is a
// routine's loft time judged against it (@FR-Perf-Weight).  No dependencies and no
// cleverness — plain idiomatic Rust, the speed an industry implementation reaches without
// effort, which is exactly what the bar should be.
//
//     rustc -O --edition=2021 bench/bench.rs -o bench/.build/stats_rs && bench/.build/stats_rs --n 2
//
// Each routine is a port of its loft original (src/math.loft, src/mesh.loft): a vector is a
// `Copy` struct, a matrix a column-major `[f64; 16]`, a mesh a `Vec<Vertex>` beside a
// `Vec<[u32; 3]>`.  A division that loft discharges with `?? 0.0` is a guarded division
// here.  `black_box` guards each op's INPUT (the repetition number, the mesh) and the sink —
// never anything inside a kernel.
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const SEG: i64 = 128;
const TRANSFORMS: i64 = 500000;
const MULS: i64 = 100000;

fn fnv(h0: i64, v: &[i64]) -> i64 {
    let mut h = h0;
    for &x in v {
        let w = x & 0xFFFF_FFFF;
        for sh in [24, 16, 8, 0] {
            h = ((h ^ ((w >> sh) & 255)) * FNV_PRIME) & 0xFFFF_FFFF;
        }
    }
    h
}

fn micro(v: f64) -> i64 {
    (v * 1000000.0) as i64
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

// ── math.loft ───────────────────────────────────────────────────────

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

fn length3(v: Vec3) -> f64 {
    (v.x * v.x + v.y * v.y + v.z * v.z).sqrt()
}

fn normalize3(v: Vec3) -> Vec3 {
    let l = length3(v);
    if l == 0.0 {
        return Vec3 { x: 0.0, y: 0.0, z: 0.0 };
    }
    Vec3 { x: v.x / l, y: v.y / l, z: v.z / l }
}

#[derive(Clone, Copy)]
struct Mat4 {
    m: [f64; 16],
}

fn mat4_identity() -> Mat4 {
    Mat4 { m: [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0] }
}

fn mat4_translate(tx: f64, ty: f64, tz: f64) -> Mat4 {
    Mat4 { m: [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, tx, ty, tz, 1.0] }
}

fn mat4_rotate_y(angle: f64) -> Mat4 {
    let (c, s) = (angle.cos(), angle.sin());
    Mat4 { m: [c, 0.0, s, 0.0, 0.0, 1.0, 0.0, 0.0, -s, 0.0, c, 0.0, 0.0, 0.0, 0.0, 1.0] }
}

fn mat4_rotate_x(angle: f64) -> Mat4 {
    let (c, s) = (angle.cos(), angle.sin());
    Mat4 { m: [1.0, 0.0, 0.0, 0.0, 0.0, c, -s, 0.0, 0.0, s, c, 0.0, 0.0, 0.0, 0.0, 1.0] }
}

fn mat4_mul(a: &Mat4, b: &Mat4) -> Mat4 {
    let mut r = Mat4 { m: [0.0; 16] };
    for col in 0..4 {
        for row in 0..4 {
            let mut sum = 0.0;
            for k in 0..4 {
                sum += a.m[k * 4 + row] * b.m[col * 4 + k];
            }
            r.m[col * 4 + row] = sum;
        }
    }
    r
}

fn mat4_transform(m: &Mat4, v: Vec3) -> Vec3 {
    let m = &m.m;
    Vec3 {
        x: m[0] * v.x + m[4] * v.y + m[8] * v.z + m[12],
        y: m[1] * v.x + m[5] * v.y + m[9] * v.z + m[13],
        z: m[2] * v.x + m[6] * v.y + m[10] * v.z + m[14],
    }
}

// ── mesh.loft ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Vertex {
    pos: Vec3,
    normal: Vec3,
    uv: Vec2,
}

struct Mesh {
    #[allow(dead_code)]
    name: String,
    vertices: Vec<Vertex>,
    triangles: Vec<[u32; 3]>,
}

impl Mesh {
    fn new(name: &str) -> Mesh {
        Mesh { name: name.to_string(), vertices: Vec::new(), triangles: Vec::new() }
    }

    fn add_vertex(&mut self, v: Vertex) -> u32 {
        let idx = self.vertices.len() as u32;
        self.vertices.push(v);
        idx
    }

    fn add_quad(&mut self, i0: u32, i1: u32, i2: u32, i3: u32) {
        self.triangles.push([i0, i1, i2]);
        self.triangles.push([i0, i2, i3]);
    }
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
            let sx = ring_r * theta.cos();
            let sz = ring_r * theta.sin();
            let su = (col as f64) / (ss as f64);
            let sv = (row as f64) / (st as f64);
            let pos = Vec3 { x: sx, y: sy, z: sz };
            sm.add_vertex(Vertex { pos, normal: normalize3(pos), uv: Vec2 { x: su, y: sv } });
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

fn mesh_to_floats(m: &Mesh) -> Vec<f32> {
    let mut buf: Vec<f32> = Vec::new();
    for tri in &m.triangles {
        for &idx in tri {
            if let Some(v) = m.vertices.get(idx as usize) {
                buf.extend_from_slice(&[v.pos.x as f32, v.pos.y as f32, v.pos.z as f32]);
                buf.extend_from_slice(&[v.normal.x as f32, v.normal.y as f32, v.normal.z as f32]);
            }
        }
    }
    buf
}

// ── The rows ────────────────────────────────────────────────────────

fn timed<F: FnMut(i64) -> i64>(n: i64, mut f: F) -> (i64, i64) {
    let t0 = Instant::now();
    let mut sink = 0i64;
    for r in 0..n {
        sink = sink.wrapping_add(f(black_box(r)));
    }
    (t0.elapsed().as_micros() as i64, black_box(sink))
}

fn sphere_op(r: i64) -> Mesh {
    sphere("ball", 1.0 + ((r & 1) as f64) * 0.5, SEG, SEG)
}

fn fnv_mesh(m: &Mesh) -> i64 {
    let mut ints: Vec<i64> = Vec::new();
    for v in &m.vertices {
        ints.extend_from_slice(&[micro(v.pos.x), micro(v.pos.y), micro(v.pos.z),
                                 micro(v.normal.x), micro(v.normal.y), micro(v.normal.z),
                                 micro(v.uv.x), micro(v.uv.y)]);
    }
    for t in &m.triangles {
        ints.extend(t.iter().map(|&i| i as i64));
    }
    fnv(FNV_OFFSET, &ints)
}

fn bench_sphere(n: i64) -> Row {
    let (us, sink) = timed(n, |r| {
        let m = sphere_op(r);
        m.triangles.len() as i64 + micro(m.vertices[1000].pos.x)
    });
    let one = sphere_op(black_box(0));
    Row { name: "sphere", iters: n, us, px: one.vertices.len() as i64, hash: fnv_mesh(&one), sink }
}

fn bench_to_floats(n: i64) -> Row {
    let a = sphere("even", 1.0, SEG, SEG);
    let b = sphere("odd", 1.5, SEG, SEG);
    let (us, sink) = timed(n, |r| {
        let out = mesh_to_floats(black_box(if (r & 1) == 0 { &a } else { &b }));
        out.len() as i64 + micro(out[7] as f64)
    });
    let one = mesh_to_floats(black_box(&a));
    let ints: Vec<i64> = one.iter().map(|&s| micro(s as f64)).collect();
    Row { name: "mesh_to_floats", iters: n, us, px: a.triangles.len() as i64,
          hash: fnv(FNV_OFFSET, &ints), sink }
}

fn orbit_matrix(r: i64) -> Mat4 {
    mat4_mul(&mat4_translate(0.25 + ((r & 1) as f64) * 0.125, -0.5, 0.75),
             &mat4_mul(&mat4_rotate_y(0.7), &mat4_rotate_x(0.3)))
}

fn transform_op(r: i64) -> [f64; 4] {
    let m = orbit_matrix(r);
    let mut p = Vec3 { x: 1.0, y: 2.0, z: 3.0 };
    let mut acc = 0.0;
    for _ in 0..TRANSFORMS {
        p = mat4_transform(&m, p);
        acc += p.x;
    }
    [p.x, p.y, p.z, acc]
}

fn fnv_floats(v: &[f64]) -> i64 {
    let ints: Vec<i64> = v.iter().map(|&f| micro(f)).collect();
    fnv(FNV_OFFSET, &ints)
}

fn bench_transform(n: i64) -> Row {
    let (us, sink) = timed(n, |r| micro(transform_op(r)[3]));
    Row { name: "mat4_transform", iters: n, us, px: TRANSFORMS,
          hash: fnv_floats(&transform_op(black_box(0))), sink }
}

fn mul_op(r: i64) -> Mat4 {
    let a = mat4_mul(&mat4_rotate_y(0.3 + ((r & 1) as f64) * 0.01), &mat4_rotate_x(0.2));
    let mut c = mat4_identity();
    for _ in 0..MULS {
        c = mat4_mul(&a, &c);
    }
    c
}

fn bench_mul(n: i64) -> Row {
    let (us, sink) = timed(n, |r| micro(mul_op(r).m[0]));
    Row { name: "mat4_mul", iters: n, us, px: MULS, hash: fnv_floats(&mul_op(black_box(0)).m), sink }
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
    let t0 = Instant::now();
    println!("routine\titers\tus\tns_op\tpx\tns_px\thash");
    let rows = [bench_to_floats(n), bench_sphere(n), bench_transform(n), bench_mul(n)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
