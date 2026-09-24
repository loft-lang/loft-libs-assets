// Copyright (c) 2026 Jurjen Stellingwerff
// SPDX-License-Identifier: LGPL-3.0-or-later
//
// assets-reference — the pure-Rust twin of the `assets` package's performance pass
// (bench/bench.loft), one file built with `rustc -O`.  It computes the SAME workloads with
// the SAME arithmetic, in the same order, and prints the same rows, hash included: a row
// whose hash matches the loft build's is a like-for-like comparison, and only then is a
// routine's loft time judged against it (@FR-Perf-Weight).  No dependencies and no
// cleverness — plain idiomatic Rust, the speed an industry implementation reaches without
// effort, which is exactly what the bar should be.
//
//     rustc -O --edition=2021 bench/bench.rs -o bench/.build/stats_rs && bench/.build/stats_rs --n 2
//
// Each routine is a port of its loft original (src/assets.loft): a keyed collection is a
// `HashMap<String, _>`, the placed `index` a `BTreeMap` over (scene, order), the texture page
// a `Vec<u8>` read through the same bounds-checked `texel_alpha`.  `keys_near` keeps the
// library's linear dedup over the (at most 40) keys found so far, so both lanes run the
// same algorithm.  `black_box` guards each op's INPUT and the sink — never anything inside a
// kernel.
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::hint::black_box;
use std::time::Instant;

const FNV_OFFSET: i64 = 2166136261;
const FNV_PRIME: i64 = 16777619;

const PAGES: i64 = 40;
const DEFS: i64 = 200;
const LEVEL: i64 = 4000;
const ATTIC: i64 = 1000;
const FACINGS: i64 = 5;
const LOOKUPS: i64 = 200000;

const CELL: i64 = 64;
const CELLS_X: i64 = 25;
const CELLS_Y: i64 = 20;
const PAGE_W: i64 = 1602;
const PROXY_CELLS: usize = 200;
const COUNT_CELLS: i64 = 500;

const BLOBS: i64 = 5000;
const BLOB_BYTES: i64 = 8192;

const PROXY_ALPHA_ON: i64 = 8;
const PROXY_BANDS: i64 = 16;

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

fn fnv_text(h0: i64, t: &str) -> i64 {
    let mut h = h0;
    for &b in t.as_bytes() {
        h = ((h ^ b as i64) * FNV_PRIME) & 0xFFFF_FFFF;
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

// ── assets.loft ─────────────────────────────────────────────────────

#[allow(dead_code)]
#[derive(Clone, Copy)]
enum BlobKind {
    Bytes,
    Rgba,
}

#[allow(dead_code)]
struct Blob {
    key: String,
    kind: BlobKind,
    bytes: Vec<u8>,
}

#[allow(dead_code)]
struct PageInfo {
    name: String,
    w: i64,
    h: i64,
    blob: String,
}

#[derive(Clone)]
#[allow(dead_code)]
struct Cell {
    key: String,
    page: String,
    x: i64,
    y: i64,
    w: i64,
    h: i64,
}

#[allow(dead_code)]
struct Def {
    name: String,
    light: bool,
    cell: String,
}

#[allow(dead_code)]
struct Anim {
    key: String,
    def: String,
    action: String,
    facing: i64,
    seq: String,
}

#[allow(dead_code)]
struct Placed {
    scene: String,
    order: i64,
    def: String,
    x: f64,
    y: f64,
}

struct Pack {
    pages: HashMap<String, PageInfo>,
    cells: HashMap<String, Cell>,
    defs: HashMap<String, Def>,
    anims: HashMap<String, Anim>,
    placed: BTreeMap<(String, i64), Placed>,
}

#[derive(Clone, Copy)]
struct Rect {
    rx: f64,
    ry: f64,
    rw: f64,
    rh: f64,
}

fn anim_of<'a>(p: &'a Pack, key: &mut String, def: &str, action: &str, facing: i64) -> &'a str {
    key.clear();
    let _ = write!(key, "{def}/{action}/{facing}");
    match p.anims.get(key.as_str()) {
        Some(row) => &row.seq,
        None => "",
    }
}

fn keys_near(p: &Pack, scene: &str, cx: f64, cy: f64, radius: f64) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for i in p.placed.values() {
        if i.scene != scene {
            continue;
        }
        if radius >= 0.0 {
            let dx = i.x - cx;
            let dy = i.y - cy;
            if dx * dx + dy * dy > radius * radius {
                continue;
            }
        }
        let Some(d) = p.defs.get(&i.def) else { continue };
        if d.cell.is_empty() {
            continue;
        }
        let Some(c) = p.cells.get(&d.cell) else { continue };
        let Some(page) = p.pages.get(&c.page) else { continue };
        if !out.iter().any(|k| *k == page.blob) {
            out.push(page.blob.clone());
        }
    }
    out
}

fn blob_put(b: &mut HashMap<String, Blob>, key: &str, kind: BlobKind, bytes: &[u8]) {
    b.insert(key.to_string(), Blob { key: key.to_string(), kind, bytes: bytes.to_vec() });
}

fn texel_alpha(px: &[u8], stride: i64, x: i64, y: i64) -> i64 {
    let i = (y * stride + x) * 4 + 3;
    if i < 0 || i >= px.len() as i64 {
        return 0;
    }
    px[i as usize] as i64
}

fn opaque_extent(px: &[u8], page: &PageInfo, c: &Cell, swap: bool) -> (i64, i64) {
    let (mut lo, mut hi) = (-1, -1);
    let vn = if swap { c.w } else { c.h };
    let un = if swap { c.h } else { c.w };
    for v in 0..vn {
        for u in 0..un {
            let ax = c.x + if swap { v } else { u };
            let ay = c.y + if swap { u } else { v };
            if texel_alpha(px, page.w, ax, ay) > PROXY_ALPHA_ON {
                if lo < 0 {
                    lo = v;
                }
                hi = v;
                break;
            }
        }
    }
    (lo, hi)
}

fn band_boxes(px: &[u8], page: &PageInfo, c: &Cell, k: i64, swap: bool) -> Vec<Rect> {
    let mut out = Vec::new();
    let (lo, hi) = opaque_extent(px, page, c, swap);
    if lo < 0 {
        return out;
    }
    let span = hi - lo + 1;
    let un = if swap { c.h } else { c.w };
    let mut min_u = vec![-1i64; k as usize];
    let mut max_u = vec![-1i64; k as usize];
    let mut min_v = vec![-1i64; k as usize];
    let mut max_v = vec![-1i64; k as usize];
    for v in lo..=hi {
        let band = (((v - lo) * k) / span).min(k - 1) as usize;
        for u in 0..un {
            let ax = c.x + if swap { v } else { u };
            let ay = c.y + if swap { u } else { v };
            if texel_alpha(px, page.w, ax, ay) <= PROXY_ALPHA_ON {
                continue;
            }
            if min_u[band] < 0 || u < min_u[band] {
                min_u[band] = u;
            }
            if u > max_u[band] {
                max_u[band] = u;
            }
            if min_v[band] < 0 || v < min_v[band] {
                min_v[band] = v;
            }
            if v > max_v[band] {
                max_v[band] = v;
            }
        }
    }
    for b in 0..k as usize {
        if min_u[b] < 0 {
            continue;
        }
        let uu = min_u[b] as f64;
        let vv = min_v[b] as f64;
        let uw = (max_u[b] - min_u[b] + 1) as f64;
        let vh = (max_v[b] - min_v[b] + 1) as f64;
        out.push(if swap {
            Rect { rx: vv, ry: uu, rw: vh, rh: uw }
        } else {
            Rect { rx: uu, ry: vv, rw: uw, rh: vh }
        });
    }
    out
}

fn boxes_area(bs: &[Rect]) -> f64 {
    let mut t = 0.0;
    for r in bs {
        t += r.rw * r.rh;
    }
    t
}

fn opaque_texels(px: &[u8], page: &PageInfo, c: &Cell) -> i64 {
    let mut n = 0;
    for y in 0..c.h {
        for x in 0..c.w {
            if texel_alpha(px, page.w, c.x + x, c.y + y) > PROXY_ALPHA_ON {
                n += 1;
            }
        }
    }
    n
}

fn cell_proxy(c: &Cell, page: &PageInfo, px: &[u8], bands: i64) -> Vec<Rect> {
    let rows = band_boxes(px, page, c, bands, false);
    let cols = band_boxes(px, page, c, bands, true);
    if !cols.is_empty() && boxes_area(&cols) < boxes_area(&rows) {
        return cols;
    }
    rows
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

const ACTIONS: [&str; 10] = ["idle", "walk", "run", "jump", "fall", "attack", "hurt", "die", "cast", "block"];

fn build_pack() -> Pack {
    let mut p = Pack { pages: HashMap::new(), cells: HashMap::new(), defs: HashMap::new(),
                       anims: HashMap::new(), placed: BTreeMap::new() };
    for pg in 0..PAGES {
        let name = format!("page_{pg}");
        p.pages.insert(name.clone(), PageInfo { name, w: 256, h: 256, blob: format!("blob_{pg}") });
    }
    for c in 0..DEFS {
        let key = format!("cell_{c}");
        p.cells.insert(key.clone(), Cell { key, page: format!("page_{}", (c * 7) % PAGES),
                                           x: 0, y: 0, w: 16, h: 16 });
    }
    for d in 0..DEFS {
        let name = format!("def_{d}");
        let def = if d % 10 == 9 {
            Def { name: name.clone(), light: true, cell: String::new() }
        } else if d % 25 == 13 {
            Def { name: name.clone(), light: false, cell: format!("gone_{d}") }
        } else {
            Def { name: name.clone(), light: false, cell: format!("cell_{d}") }
        };
        p.defs.insert(name, def);
    }
    let mut place = |scene: &str, order: i64, def: i64, x: i64, y: i64| {
        p.placed.insert((scene.to_string(), order),
                        Placed { scene: scene.to_string(), order, def: format!("def_{def}"),
                                 x: x as f64, y: y as f64 });
    };
    for i in 0..ATTIC {
        place("attic", i, (i * 13) % DEFS, (i * 31) % 1000, (i * 17) % 1000);
    }
    for i in 0..LEVEL {
        place("level", i, (i * 37) % DEFS, (i * 97) % 1000, (i * 61) % 1000);
    }
    for d in 0..DEFS {
        for a in ACTIONS {
            for f in 0..FACINGS {
                let key = format!("def_{d}/{a}/{f}");
                p.anims.insert(key.clone(), Anim { key, def: format!("def_{d}"), action: a.to_string(),
                                                   facing: f, seq: format!("def_{d}_{a}_{f}") });
            }
        }
    }
    p
}

fn bench_keys(n: i64, p: &Pack) -> Row {
    let (us, sink) = timed(n, |r| {
        keys_near(black_box(p), "level", 500.0 + ((r & 1) as f64), 500.0, 350.0).len() as i64
    });
    let one = keys_near(black_box(p), "level", 500.0, 500.0, 350.0);
    let mut h = FNV_OFFSET;
    for k in &one {
        h = fnv_text(h, k);
    }
    Row { name: "keys_near", iters: n, us, px: LEVEL + ATTIC, hash: h, sink }
}

fn anim_op(p: &Pack, defs: &[String], r: i64) -> [i64; 2] {
    let mut key = String::new();
    let mut total = 0i64;
    let mut hits = 0i64;
    for i in 0..LOOKUPS {
        let j = i + (r & 1);
        let s = anim_of(p, &mut key, &defs[(j % DEFS) as usize], ACTIONS[((j / DEFS) % 10) as usize], (j * 7) % 6);
        total += s.len() as i64 * ((i & 7) + 1);
        if !s.is_empty() {
            hits += 1;
        }
    }
    [total, hits]
}

fn bench_anim(n: i64, p: &Pack) -> Row {
    let defs: Vec<String> = (0..DEFS).map(|d| format!("def_{d}")).collect();
    let (us, sink) = timed(n, |r| anim_op(black_box(p), &defs, r)[0]);
    Row { name: "anim_of", iters: n, us, px: LOOKUPS, hash: fnv(FNV_OFFSET, &anim_op(black_box(p), &defs, 0)), sink }
}

fn blob_fill(keys: &[String], bytes: &[u8]) -> HashMap<String, Blob> {
    let mut b = HashMap::new();
    for key in keys {
        blob_put(&mut b, key, BlobKind::Rgba, bytes);
    }
    b
}

fn blob_hash(b: &HashMap<String, Blob>) -> i64 {
    let mut sum = 0i64;
    for e in b.values() {
        let h = fnv_text(FNV_OFFSET, &e.key);
        let h = fnv(h, &[e.bytes.len() as i64, e.bytes[0] as i64, e.bytes[4095] as i64, e.bytes[8191] as i64]);
        sum = (sum + h) & 0xFFFF_FFFF;
    }
    sum
}

fn blob_bytes(salt: i64) -> Vec<u8> {
    (0..BLOB_BYTES).map(|i| ((i * 31 + salt) & 255) as u8).collect()
}

fn bench_blob(n: i64) -> Row {
    let keys: Vec<String> = (0..BLOBS).map(|i| format!("blob_{i}")).collect();
    let even = blob_bytes(0);
    let odd = blob_bytes(1);
    let (us, sink) = timed(n, |r| {
        let b = blob_fill(&keys, black_box(if (r & 1) == 0 { &even } else { &odd }));
        b["blob_7"].bytes.len() as i64
    });
    Row { name: "blob_put", iters: n, us, px: BLOBS, hash: blob_hash(&blob_fill(&keys, black_box(&even))), sink }
}

fn page_alpha(x: i64, y: i64) -> i64 {
    if x >= CELLS_X * CELL {
        return 0;
    }
    let k = (y / CELL) * CELLS_X + x / CELL;
    let lx = x % CELL;
    let ly = y % CELL;
    if k % 3 == 0 && lx - ly < 2 && ly - lx < 2 {
        return 200;
    }
    let rx = 8 + (k * 7) % 22;
    let ry = 8 + (k * 11) % 22;
    let dx = lx - 32;
    let dy = ly - 32;
    let q = dx * dx * 100 / (rx * rx) + dy * dy * 100 / (ry * ry);
    if q < 90 {
        return 255;
    }
    if q < 120 {
        return (q * 5 + k) % 16;
    }
    0
}

fn build_page() -> Vec<u8> {
    let h = CELLS_Y * CELL;
    let mut px = vec![0u8; (PAGE_W * h * 4) as usize];
    for y in 0..h {
        for x in 0..PAGE_W {
            px[((y * PAGE_W + x) * 4 + 3) as usize] = page_alpha(x, y) as u8;
        }
    }
    px
}

fn build_cells(shift: i64) -> Vec<Cell> {
    (0..CELLS_X * CELLS_Y)
        .map(|k| Cell { key: format!("c{k}"), page: "atlas".to_string(), x: (k % CELLS_X) * CELL + shift,
                        y: (k / CELLS_X) * CELL, w: CELL, h: CELL })
        .collect()
}

fn proxy_op(cells: &[Cell], page: &PageInfo, px: &[u8]) -> Vec<f64> {
    let mut out = Vec::new();
    for c in &cells[..PROXY_CELLS] {
        for b in cell_proxy(c, page, px, PROXY_BANDS) {
            out.extend_from_slice(&[b.rx, b.ry, b.rw, b.rh]);
        }
    }
    out
}

fn bench_proxy(n: i64, page: &PageInfo, px: &[u8], even: &[Cell], odd: &[Cell]) -> Row {
    let (us, sink) = timed(n, |r| {
        proxy_op(black_box(if (r & 1) == 0 { even } else { odd }), page, px).len() as i64
    });
    let ints: Vec<i64> = proxy_op(black_box(even), page, px).iter().map(|&f| micro(f)).collect();
    Row { name: "cell_proxy", iters: n, us, px: PROXY_CELLS as i64, hash: fnv(FNV_OFFSET, &ints), sink }
}

fn count_op(cells: &[Cell], page: &PageInfo, px: &[u8]) -> Vec<i64> {
    cells.iter().map(|c| opaque_texels(px, page, c)).collect()
}

fn bench_count(n: i64, page: &PageInfo, px: &[u8], even: &[Cell], odd: &[Cell]) -> Row {
    let (us, sink) = timed(n, |r| count_op(black_box(if (r & 1) == 0 { even } else { odd }), page, px)[3]);
    Row { name: "opaque_texels", iters: n, us, px: COUNT_CELLS,
          hash: fnv(FNV_OFFSET, &count_op(black_box(even), page, px)), sink }
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
    let pack = build_pack();
    let page = PageInfo { name: "atlas".to_string(), w: PAGE_W, h: CELLS_Y * CELL, blob: "atlas".to_string() };
    let px = build_page();
    let even = build_cells(0);
    let odd = build_cells(1);
    let t0 = Instant::now();
    println!("routine\titers\tus\tns_op\tpx\tns_px\thash");
    let rows = [bench_keys(n, &pack), bench_proxy(n, &page, &px, &even, &odd), bench_anim(n, &pack),
                bench_blob(n), bench_count(n, &page, &px, &even, &odd)];
    let mut sink = 0i64;
    for row in &rows {
        print_row(row);
        sink = sink.wrapping_add(row.sink);
    }
    println!("time: {}ms sink={}", t0.elapsed().as_millis(), sink);
}
