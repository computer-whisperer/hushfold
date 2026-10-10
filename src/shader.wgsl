// Hushfold scene shader. Everything is a box but the paper birds, which have meshes of
// their own (gfx::crane); `kind` picks a procedural material.

struct Light {
    pos_radius: vec4<f32>,
    color: vec4<f32>,
};

struct Globals {
    view_proj: mat4x4<f32>,
    cam_pos: vec4<f32>,        // w = time
    fog: vec4<f32>,            // rgb fog colour, w density
    ambient_top: vec4<f32>,
    ambient_bottom: vec4<f32>, // w = light count
    lens: vec4<f32>,           // x = group sought (0 off, 1 dept, 2 category, 3 series), y = which
    glow: vec4<f32>,           // Kindred Glow: x = reach (m), y = strength, z = glint
    lights: array<Light, 64>,
};

@group(0) @binding(0) var<uniform> g: Globals;
// The opaque scene's depth, for translucent things (fs_trans, fs_beam).
@group(1) @binding(0) var scene_depth: texture_depth_multisampled_2d;

const K_PLAIN: u32 = 0u;
const K_WOOD: u32 = 1u;
const K_BOOK: u32 = 2u; // drawn by fs_book
const K_FLOOR: u32 = 3u;
const K_STONE: u32 = 4u;
const K_EMISSIVE: u32 = 5u;
const K_GHOST: u32 = 6u;
const K_RUG: u32 = 7u;
const K_PLAQUE: u32 = 8u;
const K_GLASS: u32 = 9u;
const K_VEIL: u32 = 10u;
const K_SHAFT: u32 = 11u; // drawn by fs_beam
const K_PAPER: u32 = 12u;
const K_METAL: u32 = 13u;
const K_CLOTH: u32 = 14u;
const K_LEAF: u32 = 15u;
const K_CRYSTAL: u32 = 16u;
const K_PARCHMENT: u32 = 17u;
const K_CRANE: u32 = 18u;

const BRASS: vec3<f32> = vec3<f32>(0.8, 0.5, 0.15);

struct VIn {
    @location(0) pos: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct IIn {
    @location(2) ipos: vec3<f32>,
    @location(3) kind: u32,
    @location(4) rot: vec4<f32>,
    @location(5) scale: vec3<f32>,
    @location(6) param: u32,
    @location(7) color: vec4<f32>,
};

struct VOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) local: vec3<f32>,
    // Per face or per instance from here on.
    @location(2) @interpolate(flat) lnormal: vec3<f32>,
    @location(3) @interpolate(flat) kind: u32,
    @location(4) @interpolate(flat) param: u32,
    @location(5) @interpolate(flat) color: vec4<f32>,
    @location(6) @interpolate(flat) scale: vec3<f32>,
    @location(7) @interpolate(flat) rot: vec4<f32>,
    @location(8) @interpolate(flat) centre: vec3<f32>,
};

fn qrot(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}

fn qrot_inv(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    return qrot(vec4<f32>(-q.xyz, q.w), v);
}

@vertex
fn vs_main(v: VIn, i: IIn) -> VOut {
    var o: VOut;
    let world = i.ipos + qrot(i.rot, v.pos * i.scale);
    o.clip = g.view_proj * vec4<f32>(world, 1.0);
    o.world = world;
    o.local = v.pos;
    o.lnormal = v.normal;
    o.kind = i.kind;
    o.param = i.param;
    o.color = i.color;
    o.scale = i.scale;
    o.rot = i.rot;
    o.centre = i.ipos;
    return o;
}

struct ModelIn {
    @location(0) at: vec3<f32>,
    @location(1) local: vec3<f32>,
    @location(8) face: vec3<f32>,
};

// Books in hand, modelled in the round (`gfx::books::model`): the material reads each
// point as the point of the box it stands for, and param's bit 31 tells it the page
// block is sunk already.
@vertex
fn vs_book_model(v: ModelIn, i: IIn) -> VOut {
    var o: VOut;
    let world = i.ipos + qrot(i.rot, v.at);
    o.clip = g.view_proj * vec4<f32>(world, 1.0);
    o.world = world;
    o.local = v.local;
    o.lnormal = v.face;
    o.kind = i.kind;
    o.param = i.param | 0x80000000u;
    o.color = i.color;
    o.scale = i.scale;
    o.rot = i.rot;
    o.centre = i.ipos;
    return o;
}

// ---------------------------------------------------------------- noise

fn hash3(p: vec3<f32>) -> f32 {
    var q = fract(p * 0.3183099 + vec3<f32>(0.71, 0.113, 0.419));
    q = q * 17.0;
    return fract(q.x * q.y * q.z * (q.x + q.y + q.z));
}

fn hash1(n: f32) -> f32 {
    return fract(sin(n * 12.9898) * 43758.5453);
}

fn vnoise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = mix(hash3(i), hash3(i + vec3<f32>(1.0, 0.0, 0.0)), u.x);
    let b = mix(hash3(i + vec3<f32>(0.0, 1.0, 0.0)), hash3(i + vec3<f32>(1.0, 1.0, 0.0)), u.x);
    let c = mix(hash3(i + vec3<f32>(0.0, 0.0, 1.0)), hash3(i + vec3<f32>(1.0, 0.0, 1.0)), u.x);
    let d = mix(hash3(i + vec3<f32>(0.0, 1.0, 1.0)), hash3(i + vec3<f32>(1.0, 1.0, 1.0)), u.x);
    return mix(mix(a, b, u.y), mix(c, d, u.y), u.z);
}

fn fbm(p: vec3<f32>) -> f32 {
    return 0.55 * vnoise(p) + 0.3 * vnoise(p * 2.07) + 0.15 * vnoise(p * 4.13);
}

// Two octaves, where a third would not be seen.
fn fbm2(p: vec3<f32>) -> f32 {
    return 0.65 * vnoise(p) + 0.35 * vnoise(p * 2.07);
}

// ---------------------------------------------------------------- lighting

struct Mat {
    albedo: vec3<f32>,
    spec: vec3<f32>,
    gloss: f32,
    // Light of its own, reflections included.
    emit: vec3<f32>,
    n: vec3<f32>,
    // How much light from behind shines through.
    back: f32,
};

// The library's lights on a surface: wrapped diffuse (plus `back` of the light behind
// it, shining through) and Blinn-Phong highlights in the colour `spec`.
fn shade(s: Mat, world: vec3<f32>) -> vec3<f32> {
    let view = normalize(g.cam_pos.xyz - world);
    var col = vec3<f32>(0.0);
    let count = u32(g.ambient_bottom.w);
    for (var li = 0u; li < count; li = li + 1u) {
        let l = g.lights[li];
        let to_l = l.pos_radius.xyz - world;
        let d2 = dot(to_l, to_l);
        let r = l.pos_radius.w;
        if (d2 > r * r) {
            continue;
        }
        let d = sqrt(d2);
        let ld = to_l / max(d, 1e-4);
        let x = d / r;
        let win = clamp(1.0 - x * x * x * x, 0.0, 1.0);
        let lc = l.color.rgb * (l.color.w * win * win / (d2 * 0.35 + 1.0));
        let ndl = dot(s.n, ld);
        let diff = max(ndl * 0.85 + 0.15, 0.0) + s.back * max(-ndl, 0.0);
        let spec = pow(max(dot(s.n, normalize(ld + view)), 0.0), s.gloss);
        col += lc * (s.albedo * diff + s.spec * spec);
    }
    return col;
}

// What shiny things see: a dark room with candlelit spots round the horizon and pale
// moonlit windows higher up. `blur` (0 to 1) smears it toward its average.
fn env(r: vec3<f32>, blur: f32) -> vec3<f32> {
    let base = mix(g.ambient_bottom.rgb * 0.4, g.ambient_top.rgb * 0.6, smoothstep(-0.4, 0.6, r.y));
    let warm = pow(max(sin(r.x * 6.0 + 1.3) * sin(r.z * 5.0 + 0.4), 0.0), 3.0) * exp(-abs(r.y - 0.05) * 3.0);
    let cool = pow(max(sin(r.x * 4.0) * sin(r.z * 4.5 + 2.0), 0.0), 6.0) * smoothstep(0.1, 0.6, r.y);
    let lit = vec3<f32>(2.6, 1.6, 0.75) * warm + vec3<f32>(0.9, 1.1, 2.0) * cool;
    return base + mix(lit, vec3<f32>(0.1, 0.075, 0.06), blur);
}

fn fresnel(nv: f32) -> f32 {
    let x = 1.0 - clamp(nv, 0.0, 1.0);
    let x2 = x * x;
    return x2 * x2 * x;
}

// Distance fog, cooler far off: near things keep the candles' warmth, the far end of a
// hall sinks into moonlit blue.
fn apply_fog(col: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    let d = length(world - g.cam_pos.xyz);
    let f = 1.0 - exp(-d * g.fog.w);
    let haze = g.fog.rgb * mix(vec3<f32>(1.0), vec3<f32>(0.85, 1.0, 1.6), smoothstep(6.0, 30.0, d));
    return mix(col, haze, f);
}

// ---------------------------------------------------------------- surface helpers

// Coverage of a line of half-width `w` at distance `d`, for a pixel `fw` across: it
// thins rather than shimmers once it is narrower than a pixel.
fn line_aa(d: f32, w: f32, fw: f32) -> f32 {
    let f = max(fw, 1e-5);
    return (1.0 - smoothstep(w - 0.5 * f, w + 0.5 * f, d)) * min(1.0, 3.0 * w / f);
}

// A face's in-plane coordinates: (x, y) on z faces, (z, y) on x faces, (x, z) on y faces.
fn face_uv(ln: vec3<f32>, p: vec3<f32>) -> vec2<f32> {
    if (abs(ln.x) > 0.5) {
        return p.zy;
    }
    if (abs(ln.y) > 0.5) {
        return p.xz;
    }
    return p.xy;
}

// The normal of the box with its edges rounded (radius `r` metres; gone where that is
// under a pixel): near an edge it leans out over it, so edges catch the light.
// `pillow` swells the faces a little, so reflections sweep across them.
fn bevel(i: VOut, r: f32, k: f32, pillow: f32, fw: f32) -> vec3<f32> {
    let e = (vec3<f32>(0.5) - abs(i.local)) * i.scale;
    let inplane = vec3<f32>(1.0) - abs(i.lnormal);
    let t = clamp(vec3<f32>(1.0) - e / r, vec3<f32>(0.0), vec3<f32>(1.0)) * inplane;
    let lean = sign(i.local) * t * (k * smoothstep(0.5 * fw, 2.0 * fw, r)) + i.local * inplane * pillow;
    return qrot(i.rot, normalize(i.lnormal + lean));
}

// ---------------------------------------------------------------- materials

// Wood: growth rings round a log axis along the box's longest side, set off its centre
// so the faces show long arcs of figure, and pores where they can be seen.
fn wood(i: VOut, fw: f32) -> vec3<f32> {
    let s = i.scale;
    let m = i.local * s;
    let seed = hash3(i.centre * 7.31);
    var along: f32;
    var a1: f32;
    var a2: f32;
    if (s.x >= s.y && s.x >= s.z) {
        along = m.x; a1 = m.y; a2 = m.z;
    } else if (s.y >= s.z) {
        along = m.y; a1 = m.x; a2 = m.z;
    } else {
        along = m.z; a1 = m.x; a2 = m.y;
    }
    let off = (vec2<f32>(seed, fract(seed * 7.13)) - 0.5) * 1.4 + vec2<f32>(0.3, -0.25);
    let warp = fbm2(vec3<f32>(along * 0.7, a1 * 5.0, a2 * 5.0) + seed * 13.0);
    let ring = fract((length(vec2<f32>(a1, a2) - off) + warp * 0.05) * 22.0);
    let late = smoothstep(0.5, 0.85, ring) * (1.0 - smoothstep(0.85, 1.0, ring));
    var pores = 0.0;
    if (fw < 0.006) {
        pores = (vnoise(vec3<f32>(along * 2.5, a1 * 140.0, a2 * 140.0)) - 0.5) * (1.0 - smoothstep(0.002, 0.006, fw));
    }
    return i.color.rgb * (0.86 + 0.16 * warp - 0.2 * late + 0.14 * pores) * (0.94 + 0.12 * seed);
}

// Floorboards along x: each board its own tone and figure, seams and nail heads that
// thin out with distance, and a scuffed, duller tread. Returns albedo and gloss scale.
fn floorboards(base: vec3<f32>, w: vec3<f32>, fw: f32) -> vec4<f32> {
    let pw = 0.19;
    let pl = 1.9;
    let row = floor(w.z / pw);
    let along = w.x + hash1(row) * pl;
    let seg = floor(along / pl);
    let id = hash1(row * 31.7 + seg * 7.3);
    let fz = fract(w.z / pw);
    let fa = fract(along / pl);
    let across = (fz - 0.5) * pw;
    let warp = fbm2(vec3<f32>(along * 1.2, row * 3.1, w.z * 6.0));
    // Flat-sawn figure: rings round a heart off to one side, below the face.
    let heart = vec2<f32>((id - 0.5) * 0.5, 0.1 + 0.3 * fract(id * 9.7));
    let ring = fract((length(vec2<f32>(across - heart.x, heart.y)) + warp * 0.03) * 26.0 + along * 0.012);
    let late = smoothstep(0.5, 0.85, ring) * (1.0 - smoothstep(0.85, 1.0, ring)) * (1.0 - smoothstep(0.004, 0.012, fw));
    var pores = 0.0;
    if (fw < 0.005) {
        pores = (vnoise(vec3<f32>(along * 3.0, w.z * 160.0, row)) - 0.5) * (1.0 - smoothstep(0.002, 0.005, fw));
    }
    var col = base * (0.72 + 0.45 * id) * (0.84 + 0.14 * warp - 0.16 * late + 0.12 * pores);
    let tread = smoothstep(0.45, 0.8, vnoise(vec3<f32>(w.x * 0.3, 0.5, w.z * 0.3)));
    col = mix(col, col * 1.15 + vec3<f32>(0.01, 0.008, 0.005), tread * 0.6);
    let seam = min(min(fz, 1.0 - fz) * pw, min(fa, 1.0 - fa) * pl);
    col *= 1.0 - 0.65 * line_aa(seam, 0.0022, fw);
    let nail = length(vec2<f32>(min(fa, 1.0 - fa) * pl - 0.035, abs(across) - 0.05));
    col = mix(col, vec3<f32>(0.03, 0.028, 0.027), line_aa(nail, 0.004, fw) * 0.8);
    return vec4<f32>(col, 1.0 - 0.6 * tread);
}

struct Stone {
    albedo: vec3<f32>,
    n: vec3<f32>,
};

// Ashlar: blocks of their own tone and warmth, chipped arrises leaning in to sunken
// mortar joints, broad weathering across the wall.
fn stone(base: vec3<f32>, w: vec3<f32>, n: vec3<f32>, fw: f32) -> Stone {
    var a: f32;
    var b: f32;
    var ta = vec3<f32>(1.0, 0.0, 0.0);
    var tb = vec3<f32>(0.0, 1.0, 0.0);
    if (abs(n.x) > 0.5) {
        a = w.z; b = w.y; ta = vec3<f32>(0.0, 0.0, 1.0);
    } else if (abs(n.z) > 0.5) {
        a = w.x; b = w.y;
    } else {
        a = w.x; b = w.z; tb = vec3<f32>(0.0, 0.0, 1.0);
    }
    let bh = 0.32;
    let bw = 0.7;
    let row = floor(b / bh);
    let aa = a + row * bw * 0.5;
    let id = hash1(row * 17.1 + floor(aa / bw) * 3.7);
    let fb = fract(b / bh);
    let fa = fract(aa / bw);
    var chip = 0.0;
    if (fw < 0.012) {
        chip = (vnoise(w * 23.0) - 0.5) * 0.014 * (1.0 - smoothstep(0.006, 0.012, fw));
    }
    let da = min(fa, 1.0 - fa) * bw + chip;
    let db = min(fb, 1.0 - fb) * bh + chip;
    let tint = mix(vec3<f32>(1.05, 1.0, 0.92), vec3<f32>(0.95, 0.98, 1.04), fract(id * 7.7));
    let mottle = fbm(vec3<f32>(a, b, id) * 6.0);
    let broad = vnoise(w * 0.5);
    var col = base * tint * (0.8 + 0.35 * id) * (0.75 + 0.35 * mottle) * (0.82 + 0.32 * broad);
    let k = smoothstep(0.003, 0.009 + fw, min(da, db));
    var o: Stone;
    o.albedo = mix(base * vec3<f32>(0.5, 0.48, 0.45), col, k);
    let la = (1.0 - smoothstep(0.006, 0.03, da)) * select(1.0, -1.0, fa < 0.5);
    let lb = (1.0 - smoothstep(0.006, 0.03, db)) * select(1.0, -1.0, fb < 0.5);
    let fade = k * (1.0 - smoothstep(0.006, 0.02, fw));
    o.n = normalize(n + (ta * la + tb * lb) * 0.6 * fade);
    return o;
}

// Distance (in metres) from a point on a box face to the nearest edge of that face.
fn box_edge(local: vec3<f32>, scale: vec3<f32>, lnormal: vec3<f32>) -> f32 {
    let e = (vec3<f32>(0.5) - abs(local)) * scale;
    let n = abs(lnormal);
    return select(select(min(e.x, e.y), min(e.x, e.z), n.y > 0.5), min(e.y, e.z), n.x > 0.5);
}

fn rug_albedo(base: vec3<f32>, second: vec3<f32>, m: vec3<f32>, s: vec3<f32>) -> vec3<f32> {
    let ex = 0.5 * s.x - abs(m.x);
    let ez = 0.5 * s.z - abs(m.z);
    let e = min(ex, ez);
    var col = base;
    if (e < 0.35) {
        col = second;
        if (e > 0.12 && e < 0.22) { col = base * 0.6; }
        let wave = sin((m.x + m.z) * 18.0) * sin((m.x - m.z) * 18.0);
        if (e > 0.15 && e < 0.19 && wave > 0.2) { col = vec3<f32>(0.6, 0.45, 0.15); }
    } else {
        let p = vec2<f32>(m.x / (s.x * 0.5), m.z / (s.z * 0.5));
        let d = abs(p.x) + abs(p.y);
        let ring = fract(d * 3.0);
        if (d < 0.55) {
            col = mix(second, base * 1.2, step(0.5, ring));
        }
        let lattice = abs(sin(m.x * 6.0) * sin(m.z * 6.0));
        col *= 0.85 + 0.2 * lattice;
    }
    let fuzz = 0.85 + 0.25 * vnoise(m * 300.0);
    return col * fuzz;
}

// A moonlit leaded window, unlit (the moon is behind it): diamond quarries of old glass
// each a little different, sky brightening upward with cloud drifting across, iron
// saddle bars, and shade toward the reveal.
fn window(i: VOut, fw: f32) -> vec3<f32> {
    let uv = face_uv(i.lnormal, i.local * i.scale);
    let ext = face_uv(i.lnormal, i.scale) * 0.5;
    let upright = abs(i.lnormal.y) < 0.5;
    let t = g.cam_pos.w;
    let q = 0.2;
    let d1 = (uv.x * 1.3 + uv.y) / q;
    let d2 = (uv.x * 1.3 - uv.y) / q;
    let pane = hash3(vec3<f32>(floor(d1), floor(d2), 3.0));
    let pitch = q / length(vec2<f32>(1.3, 1.0));
    let lead_d = min(0.5 - abs(fract(d1) - 0.5), 0.5 - abs(fract(d2) - 0.5)) * pitch;
    var lead = line_aa(lead_d, 0.006, fw);
    if (upright) {
        lead = max(lead, line_aa(abs(fract(uv.y / 0.7 + 0.5) - 0.5) * 0.7, 0.011, fw));
    }
    let up = select(0.0, uv.y / max(ext.y, 0.01), upright);
    let cloud = fbm(vec3<f32>(uv * 1.6 + vec2<f32>(t * 0.02, 0.0), t * 0.01));
    // Old glass is brighter toward the middle of each quarry.
    let dome = 1.0 - smoothstep(0.1, 0.5, lead_d / pitch);
    var c = i.color.rgb * i.color.w * (0.62 + 0.5 * cloud) * (0.88 + 0.22 * up) * (0.8 + 0.3 * pane);
    c *= 0.9 + 0.25 * dome * (1.0 - smoothstep(0.003, 0.01, fw));
    let edge = min(ext.x - abs(uv.x), ext.y - abs(uv.y));
    c *= 0.55 + 0.45 * smoothstep(0.0, 0.3, edge);
    return mix(c, vec3<f32>(0.01, 0.01, 0.014), lead);
}

// Metal: highlights and reflections in its own colour over little diffuse. Dark grey
// metals (iron) come out rough, bright or coloured ones (silver, brass, copper)
// polished; `aged` adds tarnish: verdigris on warm metals, rust on grey ones. `p`
// places the tarnish.
fn metal(s: ptr<function, Mat>, c: vec3<f32>, p: vec3<f32>, world: vec3<f32>, aged: f32) {
    let n = (*s).n;
    let view = normalize(g.cam_pos.xyz - world);
    let lum = dot(c, vec3<f32>(0.3, 0.55, 0.15));
    let sat = 1.0 - min(c.r, min(c.g, c.b)) / max(max(c.r, max(c.g, c.b)), 1e-3);
    let tarnish = smoothstep(0.5 - 0.2 * aged, 0.8 - 0.25 * aged, fbm2(p * 14.0)) * (0.12 + 0.88 * aged);
    let rough = max((1.0 - smoothstep(0.2, 0.5, lum)) * (1.0 - smoothstep(0.15, 0.4, sat)), tarnish);
    let rust = select(vec3<f32>(0.2, 0.09, 0.04), vec3<f32>(0.13, 0.26, 0.2), c.r > c.b * 1.4);
    (*s).albedo = mix(c * mix(0.05, 0.2, rough), rust, tarnish * aged);
    (*s).spec = c * mix(1.6, 0.4, rough) * (1.0 - 0.6 * tarnish);
    (*s).gloss = mix(90.0, 14.0, rough);
    let fr = fresnel(dot(n, view));
    (*s).emit = env(reflect(-view, n), rough) * mix(c, vec3<f32>(1.0), fr * 0.5) * (1.0 - 0.6 * tarnish);
}

// A sign board: the painted board lit from within with a gilt line round it, in a
// moulded brass frame rounded over from its outer edge down to the board.
fn plaque(s: ptr<function, Mat>, i: VOut, fw: f32) {
    let m = i.local * i.scale;
    let ex = 0.5 * i.scale.x - abs(m.x);
    let ey = 0.5 * i.scale.y - abs(m.y);
    let e = min(ex, ey);
    let front = abs(i.lnormal.z) > 0.5;
    if (front && e >= 0.03) {
        let gilt = line_aa(abs(e - 0.042), 0.0022, fw);
        (*s).albedo = mix(i.color.rgb * (0.85 + 0.2 * smoothstep(0.03, 0.12, e)), BRASS, gilt);
        (*s).emit = i.color.rgb * i.color.w * (1.0 - gilt);
        (*s).spec = vec3<f32>(0.06 + 0.5 * gilt);
        (*s).gloss = 30.0;
        return;
    }
    if (front) {
        let out = select(vec3<f32>(0.0, sign(m.y), 0.0), vec3<f32>(sign(m.x), 0.0, 0.0), ex < ey);
        (*s).n = qrot(i.rot, normalize(i.lnormal + out * cos(e / 0.03 * 3.14159) * 0.7));
    }
    metal(s, BRASS, m, i.world, 0.0);
    // Gilt rather than bare brass: it keeps some colour of its own away from highlights.
    (*s).albedo = BRASS * 0.45;
}

// Cloth: a weave where it can be seen, folds down the broad upright faces of thin
// hangings, velvet sheen, and an optional trim (see `kind::CLOTH`).
fn cloth(s: ptr<function, Mat>, i: VOut, fw: f32) {
    let m = i.local * i.scale;
    let ln = i.lnormal;
    let uv = face_uv(ln, m);
    let thick = dot(abs(ln), i.scale);
    let thin = min(i.scale.x, min(i.scale.y, i.scale.z));
    var col = i.color.rgb;
    // Folds hang straight down the broad upright faces of thin cloth, however it is
    // turned; placed in world space, so pieces laid over one another (a tapestry's
    // motifs) fold together.
    let flat = cross(ln, qrot_inv(i.rot, vec3<f32>(0.0, 1.0, 0.0)));
    if (thick < 0.06 && thick <= thin + 1e-4 && length(flat) > 0.7) {
        let ax = normalize(flat);
        let u = dot(i.world, qrot(i.rot, ax));
        let wobble = vnoise(vec3<f32>(u * 2.5, i.world.y * 0.8, 0.0));
        let ph = u * 30.0 + 5.0 * wobble;
        // Some folds deeper than others.
        let amp = (0.7 + 0.3 * sin(wobble * 11.0 + u * 3.0)) * (1.0 - smoothstep(0.015, 0.04, fw));
        (*s).n = qrot(i.rot, normalize(ln - ax * cos(ph) * 0.6 * amp));
        col *= 0.82 + 0.18 * sin(ph) * amp;
    } else {
        // Stuffed: edges rounded over and faces swollen, like a cushion.
        (*s).n = bevel(i, thin * 0.4, 1.0, 0.6, fw);
    }
    let weave = sin(uv.x * 1600.0) * sin(uv.y * 1600.0) * (1.0 - smoothstep(0.0008, 0.002, fw));
    col *= (0.9 + 0.15 * vnoise(m * 20.0)) * (1.0 + 0.12 * weave);
    let p = i.param;
    if (p != 0u) {
        let tc = vec3<f32>(f32(p & 255u), f32((p >> 8u) & 255u), f32((p >> 16u) & 255u)) / 255.0;
        var on = line_aa(abs(box_edge(i.local, i.scale, ln) - 0.035), 0.01, fw);
        let pat = (p >> 24u) & 3u;
        if (pat == 1u) {
            on = max(on, line_aa(abs(fract(uv.x / 0.1) - 0.5) * 0.1, 0.018, fw));
        } else if (pat == 2u) {
            let q = abs(fract(uv / 0.12) - 0.5);
            on = max(on, line_aa(q.x + q.y, 0.12, fw * 8.0));
        }
        col = mix(col, tc * tc, on);
    }
    // Velvet: light caught in the pile at grazing angles.
    let rim = 1.0 - max(dot((*s).n, normalize(g.cam_pos.xyz - i.world)), 0.0);
    (*s).albedo = col * (0.85 + 0.9 * rim * rim * rim);
    (*s).spec = vec3<f32>(0.02);
    (*s).gloss = 6.0;
}

// Leaves: a thin flat box is a blade (midrib, veins, a drier margin), a long thin one a
// stem, anything bigger a mass of foliage. Light behind shines through. Returns an
// occlusion factor (deep in foliage is darker).
fn leaf(s: ptr<function, Mat>, i: VOut, fw: f32) -> f32 {
    let m = i.local * i.scale;
    let seed = hash3(i.centre * 7.31);
    let sc = i.scale;
    let ln = i.lnormal;
    let base = i.color.rgb * mix(vec3<f32>(1.12, 1.05, 0.72), vec3<f32>(0.86, 1.0, 1.1), seed);
    let lo = min(sc.x, min(sc.y, sc.z));
    let hi = max(sc.x, max(sc.y, sc.z));
    let mid = sc.x + sc.y + sc.z - lo - hi;
    (*s).back = 0.6;
    (*s).spec = vec3<f32>(0.12);
    (*s).gloss = 22.0;
    var occl = 1.0;
    if (lo < 0.03 && mid > lo * 3.0) {
        let uv = face_uv(ln, m);
        let ext = face_uv(ln, sc) * 0.5;
        var col = base;
        if (dot(abs(ln), sc) <= lo + 1e-4) {
            let long_x = ext.x >= ext.y;
            let len = max(ext.x, ext.y);
            let u = select(uv.y, uv.x, long_x) / len;
            let v = select(uv.x, uv.y, long_x);
            let vn = abs(v) / min(ext.x, ext.y);
            let rib = line_aa(abs(v), 0.0012 + 0.0015 * (1.0 - abs(u)), fw);
            let vp = 0.3 * len;
            let vein = line_aa(abs(fract((u * len + abs(v) * 1.2) / vp) - 0.5) * vp * 0.6, 0.0007, fw);
            col = base * (0.8 + 0.3 * (1.0 - vn * vn));
            col = mix(col, base * 1.5 + vec3<f32>(0.03, 0.04, 0.0), max(rib, vein * 0.5 * (1.0 - vn)));
            col = mix(col, base * vec3<f32>(1.3, 1.1, 0.5), smoothstep(0.8, 1.0, max(vn, abs(u))) * 0.5);
            (*s).spec = vec3<f32>(0.2);
            (*s).gloss = 32.0;
        }
        (*s).albedo = col;
    } else if (hi > mid * 5.0) {
        (*s).albedo = base * (0.8 + 0.3 * vnoise(m * vec3<f32>(60.0, 8.0, 60.0)));
    } else {
        // Overlapping leaves on the face, each an ellipse at its own angle and tilt; the
        // gaps between them are the shade inside the plant.
        let cell_m = 0.045;
        let uv = face_uv(ln, m) / cell_m + seed * 17.0;
        let cell = floor(uv);
        var top = -1.0;
        var rib = 1.0;
        var edge = 1.0;
        if (fw < cell_m * 0.4) {
            for (var y = -1; y <= 1; y = y + 1) {
                for (var x = -1; x <= 1; x = x + 1) {
                    let c = cell + vec2<f32>(f32(x), f32(y));
                    let h = hash3(vec3<f32>(c, seed * 31.0));
                    let d = uv - c - 0.5 - (vec2<f32>(fract(h * 13.7), fract(h * 71.3)) - 0.5) * 0.8;
                    let dir = vec2<f32>(cos(h * 40.0), sin(h * 40.0));
                    let lu = dot(d, dir);
                    let lv = d.x * dir.y - d.y * dir.x;
                    let e = lu * lu / 0.7 + lv * lv / 0.13;
                    if (e < 1.0 && h > top) {
                        top = h;
                        rib = abs(lv);
                        edge = e;
                    }
                }
            }
        }
        let fade = 1.0 - smoothstep(cell_m * 0.15, cell_m * 0.4, fw);
        var col = base * 0.6;
        var k = 0.6;
        if (top >= 0.0) {
            col = base * (0.7 + 0.6 * top) * (1.0 - 0.25 * edge);
            col = mix(col, base * 1.4, line_aa(rib, 0.03, fw / cell_m) * 0.5);
            let tilt = vec3<f32>(fract(top * 7.1), fract(top * 3.3), fract(top * 5.9)) - 0.5;
            (*s).n = normalize((*s).n + tilt * 1.2 * fade);
            k = 1.0;
        } else if (fade > 0.0) {
            col = base * 0.18;
            k = 0.4;
        }
        (*s).albedo = mix(base * 0.6, col, fade);
        occl = mix(0.7, k, fade);
    }
    return occl;
}

// Crystal and glass: a faceted body that the room is reflected in, light from behind
// shining through it, a rim of its own colour, and color.w of light from deep inside:
// a core and flecks that shift with the view, lit edges, a slow breath. The inner
// structure shows only where the box is big enough on screen; many small pieces (a
// globe's seas, an egg's slices) glow as one.
fn crystal(s: ptr<function, Mat>, i: VOut, fw: f32) {
    let view = normalize(g.cam_pos.xyz - i.world);
    let seed = hash3(i.centre * 7.31);
    let c = i.color.rgb;
    let sc = i.scale;
    let lo = min(sc.x, min(sc.y, sc.z));
    (*s).n = bevel(i, lo * 0.3, 1.6, 0.3, fw);
    let fr = fresnel(dot((*s).n, view));
    let detail = smoothstep(12.0, 60.0, lo / max(fw, 1e-5));
    let into = normalize(qrot_inv(i.rot, -view) / sc);
    let p = i.local + into * 0.4;
    let core = 1.0 - smoothstep(0.0, 0.7, length(p));
    let t = g.cam_pos.w;
    let fleck = vnoise(p * sc * 40.0 + vec3<f32>(seed * 17.0, t * 0.15, 0.0));
    let pulse = 0.85 + 0.15 * sin(t * 1.6 + seed * 6.283);
    let edge = line_aa(box_edge(i.local, sc, i.lnormal), 0.002, fw) * detail;
    let inner = mix(0.75, 0.35 + 1.2 * core * core + 0.6 * fleck * core, detail) + 0.5 * edge;
    (*s).albedo = c * 0.3;
    (*s).back = 0.7;
    (*s).spec = vec3<f32>(1.5);
    (*s).gloss = 160.0;
    (*s).emit = env(reflect(-view, (*s).n), 0.0) * mix(c, vec3<f32>(1.0), 0.4) * (0.5 + 2.5 * fr)
        + c * g.ambient_top.rgb * 2.5 * fr
        + c * i.color.w * pulse * inner;
}

// Ink on a sheet (mode 0 writing, 1 a map, 2 a star chart, 3 none); `uv` in metres from
// the sheet's centre, `ext` its half size. Returns coverage.
fn ink(mode: u32, uv: vec2<f32>, ext: vec2<f32>, seed: f32, fw: f32) -> f32 {
    let e = min(ext.x - abs(uv.x), ext.y - abs(uv.y));
    let margin = smoothstep(0.025, 0.03, e);
    switch mode {
        case 0u: {
            let pitch = 0.016;
            let row = floor(uv.y / pitch);
            let fy = fract(uv.y / pitch);
            let wx = uv.x / 0.03 + hash1(row + seed * 50.0) * 3.0;
            let wh = hash1(floor(wx) * 7.3 + row * 1.7 + seed * 11.0);
            let word = step(fract(wx), 0.5 + 0.4 * wh);
            let strip = 1.0 - smoothstep(0.15, 0.3, abs(fy - 0.5));
            let letters = 0.55 + 0.45 * sin(uv.x * 1400.0 + wh * 9.0);
            let near = word * strip * letters;
            return mix(near, 0.2 * strip, smoothstep(0.0015, 0.004, fw)) * margin;
        }
        case 1u: {
            let h = fbm(vec3<f32>(uv * 5.0, seed * 23.0));
            let coast = line_aa(abs(h - 0.5), 0.01, fw * 4.0);
            let shore = smoothstep(0.38, 0.5, h) * (1.0 - step(0.5, h));
            let ripple = line_aa(abs(fract(h * 30.0) - 0.5) / 30.0, 0.004, fw * 4.0) * shore;
            let gq = abs(fract(uv / 0.07) - 0.5) * 0.07;
            let grat = line_aa(min(gq.x, gq.y), 0.0006, fw);
            let frame = line_aa(abs(e - 0.015), 0.0012, fw);
            let land = step(0.5, h) * 0.12;
            return max(max(coast, frame), max(0.5 * ripple, max(0.35 * grat, land)) * margin);
        }
        case 2u: {
            let r = length(uv);
            let big = min(ext.x, ext.y) * 0.85;
            var k = line_aa(abs(r - big), 0.0012, fw);
            k = max(k, line_aa(abs(r - big * 0.62), 0.0008, fw));
            k = max(k, line_aa(abs(r - big * 0.3), 0.0008, fw));
            let spoke = abs(fract(atan2(uv.y, uv.x) * 1.9099 + 0.5) - 0.5) * r * 0.5236;
            k = max(k, line_aa(spoke, 0.0005, fw) * step(r, big) * 0.6);
            let cell = floor(uv / 0.012);
            let h = hash3(vec3<f32>(cell, seed * 40.0));
            let pt = (cell + 0.5 + (vec2<f32>(fract(h * 13.7), fract(h * 71.3)) - 0.5) * 0.6) * 0.012;
            k = max(k, line_aa(length(uv - pt), 0.0006 + 0.002 * (h - 0.8), fw) * step(0.8, h));
            return max(k * margin, line_aa(abs(e - 0.015), 0.0012, fw));
        }
        default: {
            return 0.0;
        }
    }
}

// Parchment: stained and foxed, darker toward its edges, with ink on its broad faces
// (see `kind::PARCHMENT`); the ends of a rolled scroll show its turns.
fn parchment(s: ptr<function, Mat>, i: VOut, fw: f32) {
    let m = i.local * i.scale;
    let seed = hash3(i.centre * 7.31);
    let sc = i.scale;
    let ln = i.lnormal;
    let lo = min(sc.x, min(sc.y, sc.z));
    let hi = max(sc.x, max(sc.y, sc.z));
    let thick = dot(abs(ln), sc);
    let uv = face_uv(ln, m);
    let ext = face_uv(ln, sc) * 0.5;
    let stain = fbm(vec3<f32>(uv * 7.0, seed * 31.0));
    var col = i.color.rgb * (0.82 + 0.3 * stain);
    col = mix(col, col * vec3<f32>(0.8, 0.66, 0.48), smoothstep(0.6, 0.78, stain) * 0.6);
    let e = min(ext.x - abs(uv.x), ext.y - abs(uv.y));
    col *= mix(vec3<f32>(0.55, 0.42, 0.3), vec3<f32>(1.0), smoothstep(0.0, 0.035, e + (stain - 0.5) * 0.02));
    if (thick <= lo + 1e-4 && lo < 0.02) {
        col = mix(col, vec3<f32>(0.1, 0.07, 0.05), ink(i.param & 3u, uv, ext, seed, fw) * 0.85);
    } else if (thick >= hi - 1e-4 && lo >= 0.02) {
        let turn = fract(length(uv) / 0.005 - atan2(uv.y, uv.x) * 0.15915);
        col *= 1.0 - 0.3 * (1.0 - smoothstep(0.0, 0.35, turn)) * (1.0 - smoothstep(0.001, 0.003, fw));
    }
    (*s).albedo = col;
    (*s).spec = vec3<f32>(0.03);
    (*s).gloss = 10.0;
}

// A folded-paper bird (see `kind::CRANE`; its mesh's facets are lit flat, so each
// crease turns the light): the sheet's tint and print, fibres where they can be seen,
// light through the paper, and a glow from inside, strongest at the body.
fn crane(s: ptr<function, Mat>, i: VOut, fw: f32) {
    let p = i.local;
    let tint = vec3<f32>(f32((i.param >> 8u) & 255u), f32((i.param >> 16u) & 255u), f32(i.param >> 24u)) / 255.0;
    // The sheet seen from above on the wings, from the side on the body's flanks; and
    // a pixel's footprint on it (the mesh is drawn scaled).
    let n = abs(i.lnormal);
    let uv = select(select(p.zy, p.xy, n.z > n.x), p.xz, n.y > 0.5);
    let fs = fw / i.scale.x;
    let seed = dot(tint, vec3<f32>(7.1, 3.3, 5.7));
    var cover = 0.0;
    switch (i.param >> 4u) & 15u {
        case 1u, 2u, 3u: {
            cover = ink(((i.param >> 4u) & 15u) - 1u, uv, vec2<f32>(0.3), seed, fs);
        }
        case 4u: {
            // Staves of five lines, and notes on and between them.
            let row = floor(uv.y / 0.05);
            let f = (uv.y - row * 0.05 - 0.012) / 0.006;
            let lines = line_aa(abs(fract(f + 0.5) - 0.5) * 0.006, 0.0004, fs) * step(-0.5, f) * step(f, 4.5);
            let cell = floor(uv.x / 0.02);
            let h = hash3(vec3<f32>(cell, row, seed));
            let at = vec2<f32>((cell + 0.5) * 0.02, row * 0.05 + 0.012 + floor(h * 9.0) * 0.003);
            let note = line_aa(length((uv - at) * vec2<f32>(0.8, 1.2)), 0.0024, fs) * step(0.5, h);
            cover = max(lines * 0.8, note);
        }
        case 5u: {
            let q = uv / 0.03;
            let c = fract(vec2<f32>(q.x + 0.5 * floor(q.y), q.y)) - 0.5;
            cover = line_aa(length(c) * 0.03, 0.0045, fs);
        }
        case 6u: {
            // Waves: rings of scales, each row overlapping the one behind.
            let q = uv / 0.045;
            let row = floor(q.y);
            let c = vec2<f32>(fract(q.x + 0.5 * row) - 0.5, fract(q.y));
            let d = length(c) * 0.045;
            cover = line_aa(abs(fract(d / 0.009 + 0.5) - 0.5) * 0.009, 0.0012, fs) * step(d, 0.045);
        }
        case 7u: {
            cover = line_aa(abs(fract((uv.x + uv.y) / 0.05) - 0.5) * 0.05, 0.007, fs);
        }
        case 8u: {
            let c = floor(uv / 0.04);
            cover = mix(f32((i32(c.x + c.y) & 1) == 1), 0.5, smoothstep(0.01, 0.03, fs)) * 0.7;
        }
        default: {}
    }
    var fibre = 0.0;
    if (fs < 0.006) {
        fibre = (vnoise(vec3<f32>(uv * 260.0, seed)) - 0.5) * (1.0 - smoothstep(0.002, 0.006, fs));
    }
    let paper = tint * (0.94 + 0.1 * vnoise(vec3<f32>(uv * 14.0, seed)) + 0.12 * fibre);
    let dark = dot(tint, vec3<f32>(0.3, 0.55, 0.15)) < 0.4;
    let ink_rgb = select(vec3<f32>(0.1, 0.07, 0.05), vec3<f32>(0.95, 0.9, 0.78), dark);
    (*s).albedo = mix(paper, ink_rgb, cover * 0.85);
    (*s).back = 0.45;
    (*s).spec = vec3<f32>(0.05);
    (*s).gloss = 10.0;
    (*s).emit = i.color.rgb * i.color.w * (0.25 + 0.75 / (1.0 + dot(p, p) * 60.0)) * mix(vec3<f32>(1.0), tint, 0.5);
}

@fragment
fn fs_main(i: VOut) -> @location(0) vec4<f32> {
    // How much world a pixel spans here (derivatives first, in uniform control flow):
    // fine detail fades out before it can shimmer.
    let fw = length(fwidth(i.world));
    // Few values live across the switch: each material works out what it needs.
    var s: Mat;
    s.albedo = i.color.rgb;
    s.spec = vec3<f32>(0.04);
    s.gloss = 16.0;
    s.emit = vec3<f32>(0.0);
    s.n = qrot(i.rot, i.lnormal);
    s.back = 0.0;
    var ao = mix(0.55, 1.0, smoothstep(0.0, 2.2, i.world.y));
    switch i.kind {
        case K_WOOD: {
            // The bevel first: its inputs are then dead through the grain's noise.
            s.spec = vec3<f32>(0.07 + 0.05 * max(s.n.y, 0.0));
            s.n = bevel(i, 0.006, 1.0, 0.0, fw);
            s.albedo = wood(i, fw);
            s.gloss = 28.0;
        }
        case K_FLOOR: {
            let f = floorboards(i.color.rgb, i.world, fw);
            s.albedo = f.rgb;
            s.spec = vec3<f32>(0.12 * f.a);
            s.gloss = 40.0;
            ao = 1.0;
        }
        case K_STONE: {
            let st = stone(i.color.rgb, i.world, s.n, fw);
            s.albedo = st.albedo;
            s.n = st.n;
        }
        case K_EMISSIVE: {
            var flick = 1.0;
            if (i.param == 1u) {
                let t = g.cam_pos.w;
                flick = 0.8 + 0.2 * vnoise(vec3<f32>(t * 5.0, i.world.x, i.world.z));
            }
            return vec4<f32>(apply_fog(i.color.rgb * i.color.w * flick, i.world), 1.0);
        }
        case K_RUG: {
            let second = vec3<f32>(f32(i.param & 255u), f32((i.param >> 8u) & 255u), f32((i.param >> 16u) & 255u)) / 255.0;
            s.albedo = rug_albedo(i.color.rgb, second * second, i.local * i.scale, i.scale);
            ao = 1.0;
        }
        case K_PLAQUE: {
            plaque(&s, i, fw);
        }
        case K_GLASS: { // moonlit window
            return vec4<f32>(apply_fog(window(i, fw), i.world), 1.0);
        }
        case K_PAPER: {
            s.albedo = vec3<f32>(0.9, 0.86, 0.75);
            s.emit = i.color.rgb * i.color.w;
        }
        case K_METAL: {
            let lo = min(i.scale.x, min(i.scale.y, i.scale.z));
            s.n = bevel(i, clamp(lo * 0.2, 0.003, 0.03), 1.2, 0.5, fw);
            metal(&s, i.color.rgb, i.local * i.scale + i.centre, i.world, f32(i.param & 1u));
        }
        case K_CLOTH: {
            cloth(&s, i, fw);
        }
        case K_LEAF: {
            ao *= leaf(&s, i, fw);
        }
        case K_CRYSTAL: {
            crystal(&s, i, fw);
        }
        case K_PARCHMENT: {
            parchment(&s, i, fw);
        }
        case K_CRANE: {
            crane(&s, i, fw);
        }
        default: {
            // Plain stuff (wax, marble) with a soft polish.
            s.albedo = s.albedo * (0.9 + 0.15 * vnoise(i.local * i.scale * 8.0));
            s.spec = vec3<f32>(0.08);
            s.gloss = 24.0;
        }
    }
    let amb = mix(g.ambient_bottom.rgb, g.ambient_top.rgb, s.n.y * 0.5 + 0.5) * ao;
    let col = s.albedo * amb + s.emit + shade(s, i.world);
    return vec4<f32>(apply_fog(col, i.world), 1.0);
}

// ---------------------------------------------------------------- books

// The book table (gfx::books): a header, the glyph atlas's cells, volume numbers set
// roman and arabic in every face, a record per series, the glyphs its lettering points
// into, and FACE_WORDS per face.
@group(1) @binding(3) var<storage, read> book_table: array<u32>;
// Binding materials tiling every BOOK_TILE metres: r leather grain, g cloth weave,
// b marbling, a mottle.
@group(1) @binding(4) var book_tex: texture_2d<f32>;
@group(1) @binding(5) var book_sampler: sampler;
const BOOK_TILE: f32 = 0.064;
// Words per series (gfx::books::RECORD), and volume numbers set in each face
// (gfx::books::NUMERALS).
const RECORD: u32 = 10u;
const NUMERALS: u32 = 64u;
// Raised bands run from this far below the head to as far above the tail; half a
// band's width.
const BAND_END: f32 = 0.045;
const BAND_W: f32 = 0.0026;
// Lines of capitals, and of mixed case, apart in ems.
const PITCH_CAPS: f32 = 1.2;
const PITCH: f32 = 1.25;

// How far in from the spine's edge the joint's groove runs on the covers of a book `t`
// thick: past the spine's round, 0.2 t (`gfx::books::joint`).
fn joint(t: f32) -> f32 {
    return 0.2 * t + 0.0015;
}

// An sRGB-ish colour packed in a word's low bytes (`gfx::books::rgb8`).
fn unpack_rgb(w: u32) -> vec3<f32> {
    let c = vec3<f32>(f32(w & 255u), f32((w >> 8u) & 255u), f32((w >> 16u) & 255u)) / 255.0;
    return c * c;
}

// Words per face in the book table's face table (gfx::books::FACE_WORDS): its
// capitals' height, and how far its glyphs' cells reach left and right of their pens,
// in ems (f32 bits).
const FACE_WORDS: u32 = 3u;

// A glyph word's pen, in ems from the middle of its line.
fn glyph_pen(gw: u32) -> f32 {
    return f32((gw >> 14u) & 0xffffu) / 1024.0 - 32.0;
}

// The distance field (atlas pixels, inside positive) of the glyph of glyph word `gw` at
// `q`, atlas pixels along its line from the middle and up from its baseline; outside
// its cell, as far as the field reaches.
fn glyph_field(gw: u32, q: vec2<f32>, lod: f32) -> f32 {
    let c = book_table[0] + (gw & 0x3fffu) * 2u;
    let c1 = book_table[c + 1u];
    let size = vec2<f32>(f32(c1 & 255u), f32((c1 >> 8u) & 255u));
    let at = q - vec2<f32>(glyph_pen(gw) * GLYPH_EM, 0.0) - vec2<f32>(f32((c1 >> 16u) & 255u), f32(c1 >> 24u)) + 128.0;
    if (any(at < vec2<f32>(0.0)) || any(at > size)) {
        return -GLYPH_PAD;
    }
    let c0 = book_table[c];
    let uv = (vec2<f32>(f32(c0 & 0xffffu), f32(c0 >> 16u)) + vec2<f32>(at.x, size.y - at.y))
        / vec2<f32>(textureDimensions(glyph_atlas));
    return (textureSampleLevel(glyph_atlas, glyph_sampler, uv, lod).r * 255.0 - 128.0) * GLYPH_PAD / 127.0;
}

// Coverage of a block of lettering (a word of the book table: its glyphs' first index,
// count and lines) set in face `face` at `p`, in ems from the middle of its capitals,
// its lines `pitch` ems apart; `texels` is atlas pixels to a screen pixel. Letters too
// small to read fade out, as signs' do.
fn lettering(block: u32, face: u32, p: vec2<f32>, pitch: f32, texels: f32) -> f32 {
    let count = (block >> 22u) & 127u;
    let lines = f32(block >> 29u);
    if (count == 0u || texels > GLYPH_EM / 4.0 || abs(p.y) > 0.5 * lines * pitch + 0.6) {
        return 0.0;
    }
    let first = book_table[3] + (block & 0x3fffffu);
    let ft = book_table[4] + face * FACE_WORDS;
    let cap = bitcast<f32>(book_table[ft]);
    let reach = vec2<f32>(bitcast<f32>(book_table[ft + 1u]), bitcast<f32>(book_table[ft + 2u]));
    // The line the point is on, and the point in atlas pixels from its baseline.
    let l = clamp(round(0.5 * (lines - 1.0) - p.y / pitch), 0.0, lines - 1.0);
    let line = u32(l);
    let base = (0.5 * (lines - 1.0) - l) * pitch - 0.5 * cap;
    let q = vec2<f32>(p.x, p.y - base) * GLYPH_EM;
    let key = line << 16u | u32(clamp(p.x * 1024.0 + 32768.0, 0.0, 65535.0));
    // The last glyph whose pen is at or before the point (else the first).
    var lo = first;
    var hi = first + count;
    while (hi - lo > 1u) {
        let mid = (lo + hi) / 2u;
        if ((book_table[mid] >> 14u) <= key) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let lod = clamp(log2(texels), 0.0, 3.0);
    var d = -GLYPH_PAD;
    // Every glyph of the line whose cell may hold the point (kerned letters overlap,
    // and some tails run long): from that one back while their pens are near enough
    // for a cell to reach right to it, then on while near enough for one to reach left.
    var k = lo;
    loop {
        let gw = book_table[k];
        if ((gw >> 30u) != line || glyph_pen(gw) + reach.y < p.x) {
            break;
        }
        d = max(d, glyph_field(gw, q, lod));
        if (k == first) {
            break;
        }
        k = k - 1u;
    }
    for (k = lo + 1u; k < first + count; k = k + 1u) {
        let gw = book_table[k];
        if ((gw >> 30u) != line || glyph_pen(gw) + reach.x > p.x) {
            break;
        }
        d = max(d, glyph_field(gw, q, lod));
    }
    // Small letters a little bolder, as signs' are.
    let bold = 0.35 * smoothstep(GLYPH_EM / 30.0, GLYPH_EM / 10.0, texels);
    let fade = 1.0 - smoothstep(GLYPH_EM / 7.0, GLYPH_EM / 4.0, texels);
    return clamp(d / max(texels, 1e-3) + 0.5 + bold, 0.0, 1.0) * fade;
}

// A spine's tooling by its pattern (gold, or blind with trim 3), at `a` from the head
// and `b` from the tail, `x` across. Each pattern keeps a shape that reads from across
// a room: the Great Library's categories are told apart by it. With raised bands, the
// lozenges sit in the panels below the title's and the number's.
fn spine_tooling(pattern: u32, a: f32, b: f32, x: f32, sc: vec3<f32>, bands: u32, fw: f32) -> f32 {
    // Half a fillet's (a gilt rule's) width.
    let fl = 0.00042;
    let e = min(a, b);
    var t = 0.0;
    switch pattern {
        case 0u: {
            t = max(line_aa(abs(e - 0.020), fl, fw), line_aa(abs(e - 0.025), fl, fw));
        }
        case 1u: {
            // A roll: hatching between fillets near the head; a fillet near the tail.
            let roll = 1.0 - smoothstep(0.0075 - 0.5 * fw, 0.0075 + 0.5 * fw, abs(a - 0.025));
            let hatch = 0.45 + 0.55 * line_aa(abs(fract((a + x) / 0.0028) - 0.5) * 0.0028, 0.0006, fw);
            t = max(roll * hatch, line_aa(abs(b - 0.024), fl, fw));
            t = max(t, max(line_aa(abs(a - 0.0158), fl, fw), line_aa(abs(a - 0.0342), fl, fw)));
        }
        case 3u: {
            for (var k = 0; k < 3; k = k + 1) {
                t = max(t, line_aa(abs(e - 0.017 - f32(k) * 0.005), fl, fw));
            }
        }
        case 4u: {
            var at = 0.065;
            var size = 0.017;
            if (bands > 1u) {
                let span = (sc.y - 2.0 * BAND_END) / f32(bands - 1u);
                let k = clamp(floor((a - BAND_END) / span), 2.0, max(f32(bands) - 2.0, 2.0));
                at = BAND_END + span * (k + 0.5);
                size = min(size, 0.36 * span);
            }
            let dia = abs(x) / (0.5 * sc.x) * size + abs(a - at);
            t = max(line_aa(abs(dia - size), 0.0009, fw), line_aa(dia, 0.27 * size, fw));
        }
        case 5u: {
            t = line_aa(abs(a - sc.y / 3.0), 1.5 * fl, fw);
        }
        case 6u: {
            let dot = length(vec2<f32>((fract(x / 0.004 + 0.5) - 0.5) * 0.004, a - 0.033));
            t = max(line_aa(dot, 0.0011, fw), line_aa(abs(b - 0.02), fl, fw));
        }
        default: {
            // 2 and 7: their labels (the dark panel, the inlay) are the pattern.
        }
    }
    return t;
}

// Marbled paper from the materials' marbling `mb`: clouds in shades of the binding's
// colour (so a book keeps its colour in a pile), touched with the accent and a pale
// mix, under a lace of dark veins.
fn marbled(mb: f32, base: vec3<f32>, acc: vec3<f32>) -> vec3<f32> {
    var c = mix(base * 0.42, mix(base, acc, 0.35) * 0.9, smoothstep(0.2, 0.5, mb));
    c = mix(c, mix(base, vec3<f32>(0.6, 0.54, 0.42), 0.45) * 0.85, 0.8 * smoothstep(0.6, 0.78, mb));
    return mix(c, base * 0.12, smoothstep(0.86, 0.95, mb));
}

// A printed paper label, browned with age (`mottle` places the browning).
fn paper_label(age: f32, mottle: f32) -> vec3<f32> {
    return mix(vec3<f32>(0.64, 0.57, 0.42), vec3<f32>(0.5, 0.41, 0.27), age * mottle) * (0.9 + 0.15 * mottle);
}

// The colour of a book's gold by its trim: old gold, bright gold, silver; old gold too
// for the titles of books tooled blind (trim 3).
fn tool_color(trim: u32) -> vec3<f32> {
    switch trim {
        case 1u: {
            return vec3<f32>(1.0, 0.70, 0.28);
        }
        case 2u: {
            return vec3<f32>(0.78, 0.80, 0.84);
        }
        default: {
            return vec3<f32>(0.78, 0.56, 0.24);
        }
    }
}

// A book's surface as one of its faces works it out: colour, local normal, sheen, the
// coverage of tooling (gold, or blind with trim 3), of gold whatever the trim (titles,
// numbers, gilt edges: binders gilded a title even where they tooled blind) and of
// ink, how rubbed it is (scaled by age later) and how shut in from the room's light.
struct BookSurf {
    col: vec3<f32>,
    n: vec3<f32>,
    spec: f32,
    gloss: f32,
    tool: f32,
    gold: f32,
    ink: f32,
    rub: f32,
    occl: f32,
}

// Coverage of the inside of a region `e` metres in from its edge, across a pixel `fw`.
fn inside(e: f32, fw: f32) -> f32 {
    return smoothstep(-0.5 * fw, 0.5 * fw, e);
}

// The spine: `a` down from the head, `u` across.
fn book_spine(b: ptr<function, BookSurf>, i: VOut, fw: f32, tex: vec4<f32>, rec: u32, look: u32) {
    let sc = i.scale;
    let m = i.local * sc;
    let pattern = i.param & 15u;
    let binding = look & 7u;
    let a = 0.5 * sc.y - m.y;
    let u = m.x / (0.5 * sc.x);
    // Rounded: the normal turns across it, the box's face standing for its crown.
    (*b).n = vec3<f32>(0.8 * u, 0.0, 1.0);
    (*b).rub = 0.6 * smoothstep(0.8, 1.0, abs(u)) + 1.0 - smoothstep(0.0, 0.006, min(a, sc.y - a));
    let bands = (look >> 3u) & 7u;
    if (bands > 1u) {
        let span = (sc.y - 2.0 * BAND_END) / f32(bands - 1u);
        let k = clamp(round((a - BAND_END) / span), 0.0, f32(bands - 1u));
        let dd = (a - BAND_END - k * span) / BAND_W;
        // Seen from afar a band is a thread of shade: drawn less as it thins.
        let far = 1.0 - smoothstep(BAND_W, 4.0 * BAND_W, fw);
        if (abs(dd) < 1.0) {
            // Over a band: leaning toward the head above its crest, the tail below.
            (*b).n.y = -1.6 * dd * far;
            (*b).rub = max((*b).rub, 0.8 * (1.0 - abs(dd)) * far);
        } else {
            (*b).occl = 1.0 - 0.28 * (1.0 - smoothstep(1.0, 1.9, abs(dd))) * far;
        }
    }
    if (pattern == 5u) {
        (*b).col = (*b).col * (1.0 - 0.5 * inside(sc.y / 3.0 - a, fw));
    }
    (*b).tool = spine_tooling(pattern, a, sc.y - a, m.x, sc, bands, fw);
    // The label the title sits on: leather, paper, or a strip of leather inlaid.
    let label = (look >> 11u) & 3u;
    if (label != 0u) {
        let lg = unpack2x16float(book_table[rec + 5u]);
        let half_w = select(0.5 * sc.x - 0.0024, 0.27 * sc.x, label == 3u);
        let e = min(lg.x - abs(a - lg.y), half_w - abs(m.x));
        let k = inside(e, fw);
        if (label == 2u) {
            (*b).col = mix((*b).col, paper_label(f32(look >> 24u) / 255.0, tex.a), k);
            (*b).ink = line_aa(abs(e - 0.0017), 0.0003, fw);
            (*b).spec = mix((*b).spec, 0.03, k);
        } else {
            // Pattern 2's label is darker still.
            let lc = unpack_rgb(book_table[rec + 1u]) * select(1.0, 0.45, pattern == 2u);
            (*b).col = mix((*b).col, lc * (0.62 + 0.62 * tex.r), k);
            // Gilt fillets round it, the inlay's broad enough to read from afar.
            let fl = select(0.0004, 0.0007, label == 3u);
            (*b).tool = max((*b).tool, line_aa(abs(e - 0.0005 - 2.0 * fl), fl, fw));
        }
        (*b).rub = (*b).rub * (1.0 - 0.6 * k);
        (*b).occl = (*b).occl * (0.75 + 0.25 * smoothstep(0.0, 0.0008, abs(e)));
    }
    // The title, and the volume's number.
    let texel_m = fw * 0.7071 * GLYPH_EM;
    let face = book_table[rec + 9u];
    let mode = (look >> 9u) & 3u;
    let in_ink = label == 2u || binding == 3u;
    if (mode != 0u) {
        let tg = unpack2x16float(book_table[rec + 4u]);
        var p = vec2<f32>(m.x, tg.y - a);
        var pitch = PITCH_CAPS;
        if (mode == 1u) {
            p = vec2<f32>(a - tg.y, m.x);
            pitch = PITCH;
        }
        let c = lettering(book_table[rec + 3u], face, p / tg.x, pitch, texel_m / tg.x);
        (*b).ink = max((*b).ink, select(0.0, c, in_ink));
        (*b).gold = select(c, 0.0, in_ink);
    }
    let numerals = (look >> 13u) & 3u;
    if (numerals != 0u) {
        let ng = unpack2x16float(book_table[rec + 8u]);
        let volume = min((i.param >> 8u) & 255u, 63u);
        let blk = book_table[book_table[1] + (face * NUMERALS + volume) * 2u + numerals - 1u];
        let c = lettering(blk, face, vec2<f32>(m.x, ng.y - a) / ng.x, PITCH_CAPS, texel_m / ng.x);
        (*b).gold = max((*b).gold, c);
    }
}

// A cover: `fz` in from the fore-edge, `board` in from the joint, `ey` in from the head
// or tail.
fn book_cover(b: ptr<function, BookSurf>, i: VOut, fw: f32, tex: vec4<f32>, rec: u32, look: u32) {
    let sc = i.scale;
    let m = i.local * sc;
    let binding = look & 7u;
    let base = i.color.rgb;
    let fz = 0.5 * sc.z + m.z;
    let board = 0.5 * sc.z - m.z - joint(sc.x);
    let ey = 0.5 * sc.y - abs(m.y);
    let e = min(min(fz, ey), board);
    // The joint's groove, and the boards' edges rounded over (where that can be seen).
    let groove = line_aa(abs(board), 0.0011, fw);
    let near = 1.0 - smoothstep(0.001, 0.004, fw);
    (*b).n.z = (*b).n.z + clamp(board / 0.0011, -1.0, 1.0) * 0.7 * groove;
    (*b).n.y = sign(m.y) * 0.9 * (1.0 - smoothstep(0.0, 0.0016, ey)) * near;
    (*b).n.z = (*b).n.z - 0.9 * (1.0 - smoothstep(0.0, 0.0016, fz)) * near;
    (*b).occl = 1.0 - 0.35 * groove;
    (*b).rub = 0.6 * (1.0 - smoothstep(0.0, 0.0018, min(fz, ey)));
    // The fore-edge's corners: bumped round and rubbed through.
    let cd = length(max(vec2<f32>(0.011 - fz, 0.011 - ey), vec2<f32>(0.0)));
    (*b).rub = max((*b).rub, smoothstep(0.0045, 0.009, cd + 0.003 * tex.a));
    switch binding {
        case 0u: {
            // A double fillet round the board, gold on bright-gold books, else blind.
            let f = max(line_aa(abs(e - 0.009), 0.0004, fw), line_aa(abs(e - 0.0115), 0.0004, fw));
            if (((i.param >> 4u) & 3u) == 1u) {
                (*b).tool = f;
            } else {
                (*b).col = (*b).col * (1.0 - 0.45 * f);
            }
        }
        case 1u: {
            // Leather over the spine and the corners, marbled paper between.
            let strip = board - 0.14 * sc.z;
            let corner = 0.2 * sc.z - (fz + ey);
            let k = inside(min(strip, -corner * 0.7071), fw);
            if (k > 0.0) {
                let lod = clamp(log2(fw * 2800.0), 0.0, 8.0);
                let mb = textureSampleLevel(book_tex, book_sampler, vec2<f32>(m.z, m.y) / (2.0 * BOOK_TILE), lod).b;
                let paper = marbled(mb, base, unpack_rgb(book_table[rec + 2u])) * (0.9 + 0.2 * tex.a);
                (*b).col = mix((*b).col, paper, k);
                (*b).spec = mix((*b).spec, 0.04, k);
                (*b).rub = (*b).rub * (1.0 - 0.5 * k);
            }
            (*b).tool = max(line_aa(abs(strip), 0.0004, fw), line_aa(abs(corner) * 0.7071, 0.0004, fw));
        }
        case 2u: {
            // A frame pressed in blind.
            (*b).col = (*b).col * (1.0 - 0.35 * line_aa(abs(e - 0.007), 0.0006, fw));
        }
        case 4u: {
            // Paper over the boards, cloth over the spine.
            let k = inside(board - 0.12 * sc.z, fw);
            (*b).col = mix((*b).col, base * (0.78 + 0.34 * tex.a), k);
            (*b).occl = (*b).occl * (1.0 - 0.3 * line_aa(abs(board - 0.12 * sc.z), 0.0004, fw));
        }
        default: {}
    }
    // The front cover's title: stamped in gold on cloth, printed on a paper label on
    // boards.
    let blk = book_table[rec + 6u];
    if (i.lnormal.x > 0.5 && blk != 0u) {
        let cg = unpack2x16float(book_table[rec + 7u]);
        let at = vec2<f32>(-0.5 * joint(sc.x), 0.5 * sc.y - cg.y);
        let c = lettering(
            blk,
            book_table[rec + 9u],
            vec2<f32>(at.x - m.z, m.y - at.y) / cg.x,
            PITCH_CAPS,
            fw * 0.7071 * GLYPH_EM / cg.x,
        );
        if (binding == 4u) {
            let lines = f32(blk >> 29u);
            let half = vec2<f32>(0.23 * sc.z, cg.x * (0.5 * (lines - 1.0) * PITCH_CAPS + 1.0));
            let le = min(half.x - abs(m.z - at.x), half.y - abs(m.y - at.y));
            let k = inside(le, fw);
            (*b).col = mix((*b).col, paper_label(f32(look >> 24u) / 255.0, tex.a), k);
            (*b).ink = max(c, line_aa(abs(le - 0.0025), 0.0003, fw));
            (*b).rub = (*b).rub * (1.0 - 0.7 * k);
        } else {
            (*b).gold = c;
        }
    }
}

// Head, tail or fore-edge: the boards' edges either side of the page block, which is
// sunk between them and shaded where the eye's ray meets it; the fore-edge is hollowed
// to match a rounded spine.
fn book_edge(b: ptr<function, BookSurf>, i: VOut, fw: f32, tex: vec4<f32>, rec: u32, look: u32) {
    let sc = i.scale;
    let m = i.local * sc;
    let ln = i.lnormal;
    let binding = look & 7u;
    let age = f32(look >> 24u) / 255.0;
    let sid = u32(round(i.color.a));
    let inner = 0.5 * sc.x - clamp(0.07 * sc.x, 0.0015, 0.003);
    let fore = ln.z < -0.5;
    // How much of the pixel is a board's edge, or the spine's covering end on.
    var solid = inside(abs(m.x) - inner, fw);
    if (!fore) {
        solid = max(solid, inside(m.z - 0.5 * sc.z + 0.0016, fw));
    }
    let covering = (*b).col;
    if (solid < 0.999) {
        // A book modelled in the round is already sunk and hollowed.
        var hit = m;
        if ((i.param >> 31u) == 0u) {
            let vd = qrot_inv(i.rot, i.world - g.cam_pos.xyz);
            let into = max(-dot(vd, ln), 1e-4 * length(vd));
            let sq = clamp(0.011 * sc.y, 0.0022, 0.0038);
            hit = m + vd * (sq / into);
            if (fore && binding != 3u) {
                let r = hit.x / inner;
                hit = m + vd * ((sq + 0.06 * sc.x * (1.0 - r * r)) / into);
            }
        }
        // Paper, yellowing with age, its leaves' edges in fine lines.
        let along = select(hit.z, hit.y, fore);
        let lod = clamp(log2(fw * 0.7 * 2.0 * 512.0 / BOOK_TILE), 0.0, 9.0);
        let leaves = textureSampleLevel(book_tex, book_sampler, vec2<f32>(4.0 * hit.x, 0.1 * along) / BOOK_TILE, lod + 1.0).a;
        let paper = mix(vec3<f32>(0.62, 0.55, 0.41), vec3<f32>(0.48, 0.39, 0.26), age * (0.5 + 0.5 * tex.a));
        var col = paper * (0.7 + 0.45 * leaves);
        // The leaves gathered in sections, a hairline between each.
        let sections = abs(fract(hit.x / 0.0028 + hash1(f32(sid))) - 0.5) * 0.0028;
        col = col * (1.0 - 0.3 * line_aa(sections, 0.00022, fw));
        var gold = 0.0;
        let acc = unpack_rgb(book_table[rec + 2u]);
        switch (look >> 6u) & 7u {
            case 1u: {
                // Sprinkled: flecks too small to see far off, so only a tint there.
                let cell = floor(vec2<f32>(hit.x, along) / 0.0007);
                let fleck = step(0.6, hash3(vec3<f32>(cell, f32(sid))));
                let near = 1.0 - smoothstep(0.0003, 0.0009, fw);
                col = mix(col, acc, mix(0.25, 0.85 * fleck, near));
            }
            case 2u: {
                // Gilt on the head; leather on every edge.
                if (ln.y > 0.5 || binding == 0u) {
                    gold = 0.9 + 0.1 * leaves;
                }
            }
            case 3u: {
                let mb = textureSampleLevel(book_tex, book_sampler, vec2<f32>(hit.x, along) / BOOK_TILE, lod).b;
                col = marbled(mb, i.color.rgb, acc) * (0.85 + 0.25 * leaves);
            }
            case 4u: {
                col = mix(col, acc * (0.55 + 0.35 * leaves), 0.7);
            }
            default: {}
        }
        // Foxing, and dust settled on the head.
        col = col * (1.0 - 0.3 * age * smoothstep(0.66, 0.74, tex.a));
        if (ln.y > 0.5) {
            col = mix(col, vec3<f32>(0.36, 0.34, 0.31), 0.25 * age);
        }
        // Shaded under the boards' overhang.
        var occl = 0.55 + 0.45 * smoothstep(0.0, 0.004, inner - abs(hit.x));
        if (!fore) {
            // The headband, behind the spine's covering: silk worked in two colours
            // over a core, its stitches blurring together far off.
            let h = hash1(f32(sid) * 0.37);
            let c1 = select(vec3<f32>(0.45, 0.06, 0.05), vec3<f32>(0.08, 0.2, 0.1), h > 0.5);
            let c2 = select(vec3<f32>(0.7, 0.62, 0.45), vec3<f32>(0.08, 0.1, 0.3), fract(h * 7.0) > 0.7);
            let stitch = mix(step(0.5, fract(hit.x / 0.0012)), 0.5, smoothstep(0.0004, 0.0012, fw));
            let bead = clamp(1.0 - abs(hit.z - (0.5 * sc.z - 0.003)) / 0.0013, 0.0, 1.0);
            let k = inside(hit.z - (0.5 * sc.z - 0.0042), fw);
            col = mix(col, mix(c1, c2, stitch) * (0.35 + 0.65 * bead), k);
            gold = gold * (1.0 - k);
            occl = mix(occl, 0.7, k);
        }
        // Where the ray meets the inside of a board first.
        let k = inside(abs(hit.x) - inner, fw);
        (*b).col = mix(col, covering * 0.3, k);
        (*b).gold = gold * (1.0 - k);
        (*b).occl = mix(occl, 0.45, k);
        (*b).spec = 0.03;
        (*b).gloss = 10.0;
    }
    (*b).col = mix((*b).col, covering * 0.75, solid);
    (*b).gold = (*b).gold * (1.0 - solid);
    (*b).spec = mix((*b).spec, 0.05, solid);
    (*b).occl = mix((*b).occl, 1.0, solid);
    (*b).rub = 0.8 * solid;
}

// A book (kind BOOK; see `gfx::kind::BOOK` and `gfx::books`). The box is the book's
// envelope: the rounded spine and its bands, the joints, the boards' rounded edges and
// the page block sunk between them are drawn on its faces. `tex` is the binding
// materials at this point of the face. Returns how much of the room's light reaches it.
fn book(s: ptr<function, Mat>, i: VOut, fw: f32, tex: vec4<f32>) -> f32 {
    let sid = u32(round(i.color.a));
    let rec = book_table[2] + sid * RECORD;
    let look = book_table[rec];
    let binding = look & 7u;
    let age = f32(look >> 24u) / 255.0;
    let trim = (i.param >> 4u) & 3u;
    let base = i.color.rgb;
    let ln = i.lnormal;
    // What the spine and the boards are covered in. Leather keeps a soft sheen, broken
    // by its grain.
    var b: BookSurf;
    b.col = base * (0.62 + 0.62 * tex.r);
    b.spec = 0.04 + 0.08 * tex.r;
    b.gloss = 22.0;
    if (binding == 2u || binding == 4u) {
        b.col = base * (0.6 + 0.6 * tex.g);
        b.spec = 0.03;
        b.gloss = 10.0;
    } else if (binding == 3u) {
        b.col = mix(base, base * 1.5 + 0.03, 0.4) * (0.86 + 0.28 * tex.a);
        b.spec = 0.1;
        b.gloss = 16.0;
    }
    b.n = ln;
    b.tool = 0.0;
    b.gold = 0.0;
    b.ink = 0.0;
    b.rub = 0.0;
    b.occl = 1.0;
    if (ln.z > 0.5) {
        book_spine(&b, i, fw, tex, rec, look);
    } else if (abs(ln.x) > 0.5) {
        book_cover(&b, i, fw, tex, rec, look);
    } else {
        book_edge(&b, i, fw, tex, rec, look);
    }
    // Vellum is ruled and lettered by hand.
    if (binding == 3u) {
        b.ink = max(b.ink, max(b.tool, b.gold));
        b.tool = 0.0;
        b.gold = 0.0;
    }
    // Rubbed with age, unevenly: leather to its flesh side, cloth to its threads, paper
    // to the board beneath.
    let rub = clamp(b.rub * age * (0.4 + 1.2 * tex.a), 0.0, 1.0);
    var worn = mix(base * 1.4, vec3<f32>(0.4, 0.29, 0.2), 0.45);
    if (binding == 2u) {
        worn = mix(base, vec3<f32>(0.55, 0.5, 0.42), 0.5);
    } else if (binding >= 3u) {
        worn = vec3<f32>(0.3, 0.26, 0.2);
    }
    let col = mix(mix(b.col, worn, rub), vec3<f32>(0.025, 0.02, 0.018), b.ink);
    let nw = qrot(i.rot, normalize(b.n));
    (*s).n = nw;
    (*s).albedo = col;
    (*s).spec = vec3<f32>(b.spec * (1.0 - 0.7 * rub));
    (*s).gloss = b.gloss;
    // Tooling pressed in blind; gold leaf, satin, keeping a glint of its own in the
    // dimmest corner.
    let blind = trim == 3u;
    (*s).albedo = col * (1.0 - 0.55 * select(0.0, b.tool, blind) * (1.0 - rub));
    let gt = max(b.gold, select(b.tool, 0.0, blind)) * (1.0 - rub);
    if (gt > 0.002) {
        let gold = tool_color(trim);
        let view = normalize(g.cam_pos.xyz - i.world);
        (*s).albedo = mix((*s).albedo, gold * 0.32, gt);
        (*s).spec = mix((*s).spec, gold * 0.85, gt);
        (*s).gloss = mix(b.gloss, 46.0, gt);
        (*s).emit = (env(reflect(-view, nw), 0.55) * 0.45 + vec3<f32>(0.05)) * gold * gt;
    }
    // Book flags: 1 active, 2 loose, bits 2..6 department, 7..14 category.
    let flags = i.param >> 16u;
    let lens = u32(g.lens.x);
    if (lens != 0u && (flags & 2u) != 0u) {
        // The Seeker's Lens: loose books of the group sought glow, the rest fade.
        var key = sid;
        if (lens == 1u) {
            key = (flags >> 2u) & 31u;
        } else if (lens == 2u) {
            key = (flags >> 7u) & 255u;
        }
        if (key == u32(round(g.lens.y))) {
            let sig = base / max(max(base.r, base.g), max(base.b, 1e-3));
            let rim = 1.0 - smoothstep(0.002, 0.006, box_edge(i.local, i.scale, ln));
            let pulse = 0.5 + 0.5 * sin(g.cam_pos.w * 3.0);
            (*s).emit = sig * (0.25 + 0.1 * pulse + rim * 1.4);
        } else {
            (*s).albedo = (*s).albedo * 0.22;
        }
    }
    if ((flags & 1u) != 0u) {
        // The active book in hand: a bright rim along every edge, readable on any cover
        // colour, over a faint warm wash.
        let rim = 1.0 - smoothstep(0.0015, 0.0035, box_edge(i.local, i.scale, ln));
        let pulse = 0.5 + 0.5 * sin(g.cam_pos.w * 4.0);
        (*s).emit = vec3<f32>(1.0, 0.82, 0.5) * (rim * (1.3 + 0.7 * pulse) + 0.1 + 0.08 * pulse);
    }
    return b.occl;
}

// Books (kind BOOK only), in a pipeline of their own: their material needs more than
// the rest of the scene, and would otherwise cost every material its registers.
@fragment
fn fs_book(i: VOut) -> @location(0) vec4<f32> {
    // Derivatives first, in uniform control flow: the pixel's footprint, and the binding
    // materials across the face, each book's tile set apart from its neighbours'.
    let fw = length(fwidth(i.world));
    let sid = round(i.color.a);
    let place = fract(vec2<f32>(sid * 0.618034, sid * 0.7548777 + f32((i.param >> 8u) & 255u) * 0.5698403));
    let tex = textureSample(book_tex, book_sampler, face_uv(i.lnormal, i.local * i.scale) / BOOK_TILE + place);
    var s: Mat;
    s.emit = vec3<f32>(0.0);
    s.back = 0.0;
    let occl = book(&s, i, fw, tex);
    let ao = mix(0.7, 1.0, mix(0.55, 1.0, smoothstep(0.0, 2.2, i.world.y))) * occl;
    let amb = mix(g.ambient_bottom.rgb, g.ambient_top.rgb, s.n.y * 0.5 + 0.5) * ao;
    let col = s.albedo * amb + s.emit + shade(s, i.world) * (0.35 + 0.65 * occl);
    return vec4<f32>(apply_fog(col, i.world), 1.0);
}

fn erf(x: f32) -> f32 {
    return tanh(1.12838 * x + 0.10094 * x * x * x);
}

// A moonbeam's strength down its length (local y, -0.5 at the window).
fn beam_fade(y: f32) -> f32 {
    return smoothstep(-0.5, -0.4, y) * (1.0 - smoothstep(0.0, 0.5, y));
}

// A moonbeam: the light scattered along the eye's path through a soft round shaft
// (local y runs down the beam from the window), so it reads from outside and from
// within; dust drifts in it and motes glint.
// `seen`: how far the eye sees along this fragment's ray, in multiples of the distance
// to it; `facing`: how squarely the surface it meets there faces up the beam.
fn moonbeam(i: VOut, front: bool, seen: f32, facing: f32) -> vec4<f32> {
    let cam = qrot_inv(i.rot, g.cam_pos.xyz - i.centre) / i.scale;
    let inside = all(abs(cam) < vec3<f32>(0.5));
    // From outside, shade where the eye's ray enters; from inside, where it leaves.
    if (inside == front) {
        return vec4<f32>(0.0);
    }
    // Drawn without a depth test: from outside, nothing if the near side is hidden.
    if (!inside && seen < 1.0) {
        return vec4<f32>(0.0);
    }
    // The eye's ray in box coordinates is cam + ray * u, reaching this fragment at u = 1.
    let ray = i.local - cam;
    var a = i.local;
    var u1 = 1.0;
    if (inside) {
        a = cam;
    } else {
        let safe = select(ray, vec3<f32>(1e-6), abs(ray) < vec3<f32>(1e-6));
        let tt = (sign(safe) * 0.5 - i.local) / safe;
        u1 = 1.0 + max(min(tt.x, min(tt.y, tt.z)), 0.0);
    }
    let b = cam + ray * min(u1, seen);
    // Across the beam the light falls off as a Gaussian of the distance from its axis,
    // so its integral along the ray has a closed form: with x = distance / sigma,
    // x^2 = qa t^2 + 2 qb t + qc for t from 0 (a) to 1 (b).
    let d = b - a;
    let k = 4.0 / (0.45 * 0.45);
    // Where something stands in the beam, a pool of moonlight on what faces the window.
    var pool = 0.0;
    if (seen < u1) {
        pool = exp(-k * dot(b.xz, b.xz)) * beam_fade(b.y) * facing;
    }
    let qa = k * dot(d.xz, d.xz);
    let qb = k * dot(a.xz, d.xz);
    let qc = k * dot(a.xz, a.xz);
    var through = exp(-qc);
    var tc = 0.5;
    // How much of the segment the Gaussian spans: all of it when the ray runs down the
    // beam, where the nearest point is ill-defined.
    var spread = 1.0;
    if (qa > 1e-4) {
        let sa = sqrt(qa);
        let t0 = -qb / qa;
        // sqrt(pi) / 2 = 0.886.
        through = exp(-max(qc + qb * t0, 0.0)) * 0.886 / sa * (erf(sa * (1.0 - t0)) + erf(sa * t0));
        tc = clamp(t0, 0.0, 1.0);
        spread = clamp(1.5 / sa, 0.0, 1.0);
    }
    // Faded in from the window and out toward the floor: judged where the ray passes
    // nearest the axis, or along its whole length when it runs down the beam.
    let near = beam_fade(a.y + d.y * tc);
    let whole = (beam_fade(a.y + d.y * 0.17) + beam_fade(a.y + d.y * 0.5) + beam_fade(a.y + d.y * 0.83)) / 3.0;
    let dens = through * mix(near, whole, spread);
    let len = length(d * i.scale);
    let t = g.cam_pos.w;
    // Brighter looking up the beam toward the window.
    let axis = qrot(i.rot, vec3<f32>(0.0, 1.0, 0.0));
    let fwd = 1.0 + 1.2 * pow(max(dot(normalize(i.world - g.cam_pos.xyz), -axis), 0.0), 6.0);
    let mid = i.centre + qrot(i.rot, (a + d * mix(tc, 0.5, spread)) * i.scale);
    let dust = 0.65 + 0.7 * vnoise(mid * 1.6 + vec3<f32>(t * 0.02, -t * 0.04, 0.0));
    // Motes: points in the air near the beam's skin, twinkling as they turn.
    let mp = i.world * 9.0 + vec3<f32>(0.0, -t * 0.06, 0.0);
    let cell = floor(mp);
    let h = hash3(cell);
    let pt = cell + 0.5 + (vec3<f32>(fract(h * 13.7), fract(h * 71.3), fract(h * 37.1)) - 0.5) * 0.6;
    let mote = step(0.9, h) * (1.0 - smoothstep(0.0, 0.08, length(mp - pt))) * (0.5 + 0.5 * sin(t * 2.0 + h * 60.0));
    let across_mid = 1.0 - smoothstep(0.15, 1.0, length((a + b).xz));
    let glow = i.color.w * (0.9 * dens * len * fwd * dust + 30.0 * mote * across_mid + 4.0 * pool);
    return vec4<f32>(i.color.rgb * glow, 0.0);
}

// Moonbeams, drawn with no depth test: `moonbeam` hides and clips itself against the
// scene's depth.
@fragment
fn fs_beam(i: VOut, @builtin(front_facing) front: bool) -> @location(0) vec4<f32> {
    // View depth is p23 / (z + p22) from the projection's third row; p22 is recovered
    // from view_proj, whose fourth row is the view's (unit) depth axis.
    let r2 = vec3<f32>(g.view_proj[0][2], g.view_proj[1][2], g.view_proj[2][2]);
    let r3 = vec3<f32>(g.view_proj[0][3], g.view_proj[1][3], g.view_proj[2][3]);
    let p22 = -length(r2) / length(r3);
    let seen = (i.clip.z + p22) / (textureLoad(scene_depth, vec2<i32>(i.clip.xy), 0) + p22);
    // The scene's surface there, and its normal from how that point moves across the
    // screen (derivatives here, in uniform control flow).
    let hit = g.cam_pos.xyz + (i.world - g.cam_pos.xyz) * seen;
    var n = normalize(cross(dpdx(hit), dpdy(hit)) + vec3<f32>(0.0, 1e-6, 0.0));
    n = select(n, -n, dot(n, g.cam_pos.xyz - hit) < 0.0);
    let facing = max(dot(n, -qrot(i.rot, vec3<f32>(0.0, 1.0, 0.0))), 0.0);
    return moonbeam(i, front, seen, facing);
}

// Translucent things: slot ghosts and the veil. Premultiplied alpha.
@fragment
fn fs_trans(i: VOut) -> @location(0) vec4<f32> {
    // Ghost outline distance, and its screen-space rate (derivatives need uniform control flow).
    let edge = box_edge(i.local, i.scale, i.lnormal);
    let px = max(fwidth(edge), 1e-5);

    let t = g.cam_pos.w;
    if (i.kind == K_VEIL) {
        // Noise along the veil's long side and up it; color.w fades it out.
        let m = i.local * i.scale;
        let along = select(m.z, m.x, i.scale.x > i.scale.z);
        let w = vec2<f32>(along, i.world.y);
        let n1 = fbm(vec3<f32>(w.x * 0.6, w.y * 0.4 - t * 0.15, t * 0.1));
        let n2 = fbm(vec3<f32>(w.x * 2.0 + t * 0.2, w.y * 1.5, 3.0));
        let a = (0.12 + 0.3 * n1 * n2) * i.color.w;
        let c = mix(vec3<f32>(0.2, 0.25, 0.7), vec3<f32>(0.55, 0.3, 0.8), n2) * (0.3 + 0.7 * n1);
        return vec4<f32>(c * a, a * 0.7);
    }
    // Ghost slot. param: 0 = waiting, 1 = a series you carry, 2 = the active book's slot.
    // Highlighted ghosts arrive in a fixed-brightness signal colour; color.w scales them.
    let level = i.param & 3u;
    if (level == 0u) {
        let outline = 1.0 - smoothstep(0.0015, 0.004, edge);
        let a = i.color.w * (0.25 + 0.75 * outline);
        let c = i.color.rgb * (0.35 + 1.0 * outline);
        return vec4<f32>(c * a, a * 0.6);
    }
    // Lines at least ~1.5 px wide, so distant runs still read.
    let outline = 1.0 - smoothstep(max(0.003, px * 1.5), max(0.006, px * 2.5), edge);
    let k = i.color.w;
    if (level == 1u) {
        // Kindred Glow: fading out toward its reach to about a waiting outline (a
        // little brighter when aimed at: k is 1.3 then); at full strength a slow
        // glint sweeps through every run you can file.
        let near = 1.0 - smoothstep(g.glow.x * 0.7, g.glow.x, distance(i.world, g.cam_pos.xyz));
        let kg = k * max(g.glow.y * near, select(0.015, 0.1, k > 1.1));
        let ph = (i.world.x + i.world.z) * 1.7 + i.world.y * 2.3 - t * 2.2;
        let glint = g.glow.z * pow(0.5 + 0.5 * sin(ph), 12.0);
        let c = i.color.rgb * kg * (outline * (0.55 + 0.8 * glint) + 0.05);
        return vec4<f32>(c, kg * (0.1 + 0.3 * outline));
    }
    let pulse = 0.5 + 0.5 * sin(t * 5.0);
    let c = i.color.rgb * k * (outline * (1.4 + 0.8 * pulse) + 0.3 + 0.3 * pulse);
    return vec4<f32>(c, k * (0.35 + 0.3 * outline));
}

// ---------------------------------------------------------------- lettering

// Signs and labels (text.rs): glyph quads over the surfaces they are set on, cut from
// a signed-distance atlas.
@group(1) @binding(1) var glyph_atlas: texture_2d<f32>;
@group(1) @binding(2) var glyph_sampler: sampler;
// The atlas's pixels to the em, and how far its field reaches either side of an
// outline, in its pixels (text::EM_PX, text::PAD).
const GLYPH_EM: f32 = 64.0;
const GLYPH_PAD: f32 = 8.0;
// What letters are made of (text::Lettering).
const L_GILT: u32 = 0u;
const L_GLOW: u32 = 2u;

struct GlyphIn {
    @location(0) at: vec3<f32>,
    @location(1) lettering: u32,
    @location(2) right: vec3<f32>,
    @location(3) up: vec3<f32>,
    @location(4) uv: vec4<f32>,
    @location(5) color: vec4<f32>,
};

struct GlyphOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) n: vec3<f32>,
    @location(3) @interpolate(flat) lettering: u32,
    @location(4) @interpolate(flat) color: vec4<f32>,
};

@vertex
fn vs_text(@builtin(vertex_index) v: u32, i: GlyphIn) -> GlyphOut {
    // A strip: bottom left, bottom right, top left, top right.
    let c = vec2<f32>(f32(v & 1u), f32(v >> 1u));
    var o: GlyphOut;
    o.world = i.at + i.right * c.x + i.up * c.y;
    o.clip = g.view_proj * vec4<f32>(o.world, 1.0);
    o.uv = vec2<f32>(mix(i.uv.x, i.uv.z, c.x), mix(i.uv.w, i.uv.y, c.y));
    o.n = normalize(cross(i.right, i.up));
    o.lettering = i.lettering;
    o.color = i.color;
    return o;
}

@fragment
fn fs_text(i: GlyphOut) -> @location(0) vec4<f32> {
    // Atlas pixels to a screen pixel, and the distance to the outline in atlas pixels
    // (inside positive).
    let texels = length(fwidth(i.uv * vec2<f32>(textureDimensions(glyph_atlas)))) * 0.7071;
    let d = (textureSample(glyph_atlas, glyph_sampler, i.uv).r * 255.0 - 128.0) * GLYPH_PAD / 127.0;
    // Small letters are set a little bolder (by up to a third of a pixel), and those
    // too small to read fade away rather than shimmer.
    let bold = 0.35 * smoothstep(GLYPH_EM / 30.0, GLYPH_EM / 10.0, texels);
    let fade = 1.0 - smoothstep(GLYPH_EM / 7.0, GLYPH_EM / 4.0, texels);
    let a = clamp(d / max(texels, 1e-3) + 0.5 + bold, 0.0, 1.0) * fade;
    if (a < 0.004) {
        discard;
    }
    if (i.lettering == L_GLOW) {
        return vec4<f32>(apply_fog(i.color.rgb * i.color.w, i.world) * a, a);
    }
    var s: Mat;
    s.albedo = i.color.rgb;
    s.spec = vec3<f32>(0.03);
    s.gloss = 12.0;
    s.emit = vec3<f32>(0.0);
    s.n = i.n;
    s.back = 0.0;
    if (i.lettering == L_GILT) {
        // Gold leaf, satin rather than mirror, so a name reads the same from any side;
        // it keeps a glint of its own in the dimmest corner.
        let view = normalize(g.cam_pos.xyz - i.world);
        s.albedo = i.color.rgb * 0.35;
        s.spec = i.color.rgb * 0.5;
        s.gloss = 24.0;
        s.emit = (env(reflect(-view, s.n), 0.7) * 0.5 + vec3<f32>(0.08)) * i.color.rgb;
    }
    let amb = mix(g.ambient_bottom.rgb, g.ambient_top.rgb, s.n.y * 0.5 + 0.5);
    let col = s.albedo * amb + s.emit + shade(s, i.world);
    return vec4<f32>(apply_fog(col, i.world) * a, a);
}
