// Hushfold scene shader. Everything is a box; `kind` picks a procedural material.

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

const K_PLAIN: u32 = 0u;
const K_WOOD: u32 = 1u;
const K_BOOK: u32 = 2u;
const K_FLOOR: u32 = 3u;
const K_STONE: u32 = 4u;
const K_EMISSIVE: u32 = 5u;
const K_GHOST: u32 = 6u;
const K_RUG: u32 = 7u;
const K_PLAQUE: u32 = 8u;
const K_GLASS: u32 = 9u;
const K_VEIL: u32 = 10u;
const K_SHAFT: u32 = 11u;
const K_PAPER: u32 = 12u;

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
    @location(1) normal: vec3<f32>,
    @location(2) local: vec3<f32>,
    @location(3) lnormal: vec3<f32>,
    @location(4) @interpolate(flat) kind: u32,
    @location(5) @interpolate(flat) param: u32,
    @location(6) color: vec4<f32>,
    @location(7) scale: vec3<f32>,
};

fn qrot(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}

@vertex
fn vs_main(v: VIn, i: IIn) -> VOut {
    var o: VOut;
    let world = i.ipos + qrot(i.rot, v.pos * i.scale);
    o.clip = g.view_proj * vec4<f32>(world, 1.0);
    o.world = world;
    o.normal = qrot(i.rot, v.normal);
    o.local = v.pos;
    o.lnormal = v.normal;
    o.kind = i.kind;
    o.param = i.param;
    o.color = i.color;
    o.scale = i.scale;
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

// ---------------------------------------------------------------- lighting

fn shade(albedo: vec3<f32>, n: vec3<f32>, world: vec3<f32>, spec_amt: f32, gloss: f32, ao: f32) -> vec3<f32> {
    let hemi = mix(g.ambient_bottom.rgb, g.ambient_top.rgb, n.y * 0.5 + 0.5);
    var col = albedo * hemi * ao;
    let view = normalize(g.cam_pos.xyz - world);
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
        let att = win * win / (d2 * 0.35 + 1.0);
        let diff = max(dot(n, ld) * 0.85 + 0.15, 0.0);
        let h = normalize(ld + view);
        let spec = pow(max(dot(n, h), 0.0), gloss) * spec_amt;
        col += l.color.rgb * l.color.w * att * (albedo * diff + vec3<f32>(spec));
    }
    return col;
}

fn apply_fog(col: vec3<f32>, world: vec3<f32>) -> vec3<f32> {
    let d = length(world - g.cam_pos.xyz);
    let f = 1.0 - exp(-d * g.fog.w);
    return mix(col, g.fog.rgb, f);
}

// ---------------------------------------------------------------- materials

fn wood(base: vec3<f32>, m: vec3<f32>, s: vec3<f32>) -> vec3<f32> {
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
    let warp = fbm(vec3<f32>(along * 0.8, a1 * 6.0, a2 * 6.0));
    let streak = 0.5 + 0.5 * sin((a1 + a2) * 70.0 + warp * 9.0);
    let fine = vnoise(vec3<f32>(along * 3.0, a1 * 120.0, a2 * 120.0));
    return base * (0.72 + 0.22 * streak + 0.12 * fine);
}

fn digit_mask(d: u32) -> u32 {
    switch d {
        case 0u: { return 31599u; } // 111 101 101 101 111
        case 1u: { return 11415u; } // 010 110 010 010 111
        case 2u: { return 29671u; } // 111 001 111 100 111
        case 3u: { return 29647u; } // 111 001 111 001 111
        case 4u: { return 23497u; } // 101 101 111 001 001
        case 5u: { return 31183u; } // 111 100 111 001 111
        case 6u: { return 31215u; } // 111 100 111 101 111
        case 7u: { return 29330u; } // 111 001 010 010 010
        case 8u: { return 31727u; } // 111 101 111 101 111
        default: { return 31695u; } // 111 101 111 001 111
    }
}

// Returns 1 if the spine-space point (x across, y down from top of text) is on a glyph pixel.
fn number_px(n: u32, x: f32, y: f32, ps: f32) -> f32 {
    var digits = 1u;
    if (n >= 10u) { digits = 2u; }
    let w = f32(digits * 4u - 1u) * ps;
    let gx = (x + w * 0.5) / ps;
    let gy = y / ps;
    if (gx < 0.0 || gy < 0.0 || gy >= 5.0 || gx >= f32(digits * 4u - 1u)) {
        return 0.0;
    }
    let cell = u32(gx) / 4u;
    let px = u32(gx) % 4u;
    if (px == 3u) { return 0.0; }
    var d = n % 10u;
    if (digits == 2u && cell == 0u) { d = n / 10u; }
    let bit = 14u - (u32(gy) * 3u + px);
    return f32((digit_mask(d) >> bit) & 1u);
}

fn trim_color(t: u32, base: vec3<f32>) -> vec3<f32> {
    switch t {
        case 1u: { return vec3<f32>(0.85, 0.52, 0.16); }
        case 2u: { return vec3<f32>(0.62, 0.64, 0.68); }
        case 3u: { return vec3<f32>(0.02, 0.018, 0.015); }
        default: { return vec3<f32>(0.80, 0.70, 0.48); }
    }
}

// Distance (in metres) from a point on a box face to the nearest edge of that face.
fn box_edge(local: vec3<f32>, scale: vec3<f32>, lnormal: vec3<f32>) -> f32 {
    let e = (vec3<f32>(0.5) - abs(local)) * scale;
    let n = abs(lnormal);
    return select(select(min(e.x, e.y), min(e.x, e.z), n.y > 0.5), min(e.y, e.z), n.x > 0.5);
}

struct Surf {
    albedo: vec3<f32>,
    spec: f32,
    gloss: f32,
    emit: vec3<f32>,
};

fn book_surface(i: VOut) -> Surf {
    var s: Surf;
    s.spec = 0.05;
    s.gloss = 24.0;
    s.emit = vec3<f32>(0.0);
    let base = i.color.rgb;
    let pattern = i.param & 15u;
    let trim = (i.param >> 4u) & 3u;
    let volume = (i.param >> 8u) & 255u;
    let flags = i.param >> 16u;
    let m = i.local * i.scale;
    let ln = i.lnormal;
    let paper = vec3<f32>(0.66, 0.58, 0.42);
    let cover_t = 0.0025;
    let tc = trim_color(trim, base);
    let is_metal = trim == 1u || trim == 2u;

    if (ln.z > 0.5) {
        // Spine.
        let h = i.scale.y;
        let v = m.y;
        let top = h * 0.5 - v; // distance from top
        let bot = v + h * 0.5; // distance from bottom
        var col = base;
        var metal = 0.0;
        var band = 0.0;
        switch pattern {
            case 0u: {
                band = f32(abs(top - 0.028) < 0.0025 || abs(bot - 0.028) < 0.0025);
            }
            case 1u: {
                band = f32(top > 0.02 && top < 0.05);
                band = max(band, f32(bot > 0.018 && bot < 0.026));
            }
            case 2u: {
                let panel = top > 0.04 && top < 0.11;
                if (panel) { col = base * 0.35; }
                band = f32(abs(top - 0.04) < 0.002 || abs(top - 0.11) < 0.002);
            }
            case 3u: {
                for (var k = 0; k < 3; k = k + 1) {
                    let o = 0.018 + f32(k) * 0.009;
                    band = max(band, f32(abs(top - o) < 0.0018 || abs(bot - o) < 0.0018));
                }
            }
            case 4u: {
                let c = top - 0.07;
                let dia = abs(m.x) / max(i.scale.x * 0.5, 0.001) * 0.02 + abs(c);
                band = f32(dia < 0.022 && dia > 0.016);
                band = max(band, f32(dia < 0.008));
            }
            case 5u: {
                if (top < h * 0.33) { col = base * 0.5; }
                band = f32(abs(top - h * 0.33) < 0.002);
            }
            case 6u: {
                let row = abs(top - 0.035) < 0.004;
                let dots = fract(m.x / 0.008 + 0.5) < 0.5;
                band = f32(row && dots);
                band = max(band, f32(abs(bot - 0.02) < 0.002));
            }
            default: {
                band = f32(abs(m.x) < i.scale.x * 0.12 && top > 0.02 && bot > 0.07);
            }
        }
        if (band > 0.5) {
            col = tc;
            metal = 1.0;
        }
        // Volume number near the bottom.
        if (volume > 0u) {
            var ps = 0.0032;
            var digits = 1.0;
            if (volume >= 10u) { digits = 2.0; }
            ps = min(ps, i.scale.x * 0.8 / (digits * 4.0 - 1.0));
            // Text block sits 3cm above the tail; glyph rows grow downward.
            let y_text = (0.03 + 5.0 * ps) - bot;
            let on = number_px(volume, m.x, y_text, ps);
            if (on > 0.5) {
                col = tc;
                metal = 1.0;
            }
        }
        // Rounded spine.
        let r = abs(i.local.x) * 2.0;
        col *= 1.0 - 0.35 * r * r;
        s.albedo = col;
        if (metal > 0.5 && is_metal) {
            s.spec = 0.9;
            s.gloss = 40.0;
        } else {
            s.spec = 0.06;
            s.gloss = 18.0;
        }
        // Leather/cloth grain.
        s.albedo *= 0.9 + 0.2 * vnoise(m * 900.0);
    } else if (abs(ln.x) > 0.5) {
        // Front/back cover.
        var col = base * 0.85;
        let ez = 0.5 * i.scale.z - abs(m.z);
        let ey = 0.5 * i.scale.y - abs(m.y);
        let frame = min(ez, ey);
        if (trim != 0u && frame > 0.010 && frame < 0.0135) {
            col = tc;
            if (is_metal) { s.spec = 0.6; s.gloss = 36.0; }
        }
        s.albedo = col * (0.9 + 0.2 * vnoise(m * 700.0));
    } else {
        // Paper (fore-edge, head, tail) with covers framing it.
        let edge_x = 0.5 * i.scale.x - abs(m.x);
        let spine_side = 0.5 * i.scale.z - m.z;
        var is_cover = edge_x < cover_t;
        if (abs(ln.y) > 0.5 && spine_side < 0.004) {
            is_cover = true;
        }
        if (is_cover) {
            s.albedo = base * 0.8;
        } else {
            let lines = 0.9 + 0.1 * sin(m.x * 2600.0);
            // Pages are slightly shadowed near the covers.
            let shadow = smoothstep(0.0, 0.01, edge_x - cover_t);
            s.albedo = paper * lines * (0.6 + 0.4 * shadow);
        }
    }
    // Book flags: 1 active, 2 loose, bits 2..6 department, 7..14 category; the
    // series id rides in color.a.
    let lens = u32(g.lens.x);
    if (lens != 0u && (flags & 2u) != 0u) {
        // The Seeker's Lens: loose books of the group sought glow, the rest fade.
        var key = u32(round(i.color.a));
        if (lens == 1u) {
            key = (flags >> 2u) & 31u;
        } else if (lens == 2u) {
            key = (flags >> 7u) & 255u;
        }
        if (key == u32(round(g.lens.y))) {
            let sig = base / max(max(base.r, base.g), max(base.b, 1e-3));
            let rim = 1.0 - smoothstep(0.002, 0.006, box_edge(i.local, i.scale, ln));
            let pulse = 0.5 + 0.5 * sin(g.cam_pos.w * 3.0);
            s.emit = sig * (0.25 + 0.1 * pulse + rim * 1.4);
        } else {
            s.albedo = s.albedo * 0.22;
        }
    }
    if ((flags & 1u) != 0u) {
        // The active book in hand: a bright rim along every edge, readable on any cover
        // colour, over a faint warm wash.
        let t = g.cam_pos.w;
        let rim_edge = box_edge(i.local, i.scale, ln);
        let rim = 1.0 - smoothstep(0.0015, 0.0035, rim_edge);
        let pulse = 0.5 + 0.5 * sin(t * 4.0);
        s.emit = vec3<f32>(1.0, 0.82, 0.5) * (rim * (1.3 + 0.7 * pulse) + 0.1 + 0.08 * pulse);
    }
    return s;
}

fn floor_albedo(base: vec3<f32>, w: vec3<f32>) -> vec3<f32> {
    let pw = 0.19;
    let pl = 1.9;
    let row = floor(w.z / pw);
    let off = hash1(row) * pl;
    let along = w.x + off;
    let seg = floor(along / pl);
    let id = hash1(row * 31.7 + seg * 7.3);
    let fz = fract(w.z / pw);
    let fa = fract(along / pl);
    let grain = fbm(vec3<f32>(along * 1.5, row * 3.1, w.z * 40.0));
    let streak = 0.5 + 0.5 * sin(w.z * 300.0 + grain * 10.0 + id * 20.0);
    var col = base * (0.7 + 0.5 * id) * (0.8 + 0.15 * streak + 0.1 * grain);
    let seam = min(min(fz, 1.0 - fz) * pw, min(fa, 1.0 - fa) * pl);
    col *= mix(0.35, 1.0, smoothstep(0.0, 0.004, seam));
    return col;
}

fn stone_albedo(base: vec3<f32>, w: vec3<f32>, n: vec3<f32>) -> vec3<f32> {
    var a: f32;
    var b: f32;
    if (abs(n.x) > 0.5) {
        a = w.z; b = w.y;
    } else if (abs(n.z) > 0.5) {
        a = w.x; b = w.y;
    } else {
        a = w.x; b = w.z;
    }
    let bh = 0.32;
    let bw = 0.7;
    let row = floor(b / bh);
    let aa = a + row * bw * 0.5;
    let col_i = floor(aa / bw);
    let id = hash1(row * 17.1 + col_i * 3.7);
    let fb = fract(b / bh);
    let fa = fract(aa / bw);
    let mortar = min(min(fb, 1.0 - fb) * bh, min(fa, 1.0 - fa) * bw);
    var col = base * (0.8 + 0.35 * id) * (0.75 + 0.35 * fbm(vec3<f32>(a, b, id) * 6.0));
    col = mix(base * 0.35, col, smoothstep(0.004, 0.012, mortar));
    return col;
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

@fragment
fn fs_main(i: VOut) -> @location(0) vec4<f32> {
    let n = normalize(i.normal);
    let m = i.local * i.scale;
    var albedo = i.color.rgb;
    var spec = 0.04;
    var gloss = 16.0;
    var emit = vec3<f32>(0.0);
    var ao = mix(0.55, 1.0, smoothstep(0.0, 2.2, i.world.y));
    switch i.kind {
        case 1u: { // K_WOOD
            albedo = wood(i.color.rgb, m, i.scale);
            spec = 0.08;
            gloss = 28.0;
        }
        case 2u: { // K_BOOK
            let s = book_surface(i);
            albedo = s.albedo;
            spec = s.spec;
            gloss = s.gloss;
            emit = s.emit;
            ao = mix(0.7, 1.0, ao);
        }
        case 3u: { // K_FLOOR
            albedo = floor_albedo(i.color.rgb, i.world);
            spec = 0.12;
            gloss = 40.0;
            ao = 1.0;
        }
        case 4u: { // K_STONE
            albedo = stone_albedo(i.color.rgb, i.world, n);
        }
        case 5u: { // K_EMISSIVE
            var flick = 1.0;
            if (i.param == 1u) {
                let t = g.cam_pos.w;
                flick = 0.8 + 0.2 * vnoise(vec3<f32>(t * 5.0, i.world.x, i.world.z));
            }
            return vec4<f32>(apply_fog(i.color.rgb * i.color.w * flick, i.world), 1.0);
        }
        case 7u: { // K_RUG
            let second = vec3<f32>(f32(i.param & 255u), f32((i.param >> 8u) & 255u), f32((i.param >> 16u) & 255u)) / 255.0;
            albedo = rug_albedo(i.color.rgb, second * second, m, i.scale);
            ao = 1.0;
        }
        case 8u: { // K_PLAQUE
            let ex = 0.5 * i.scale.x - abs(m.x);
            let ey = 0.5 * i.scale.y - abs(m.y);
            let e = min(ex, ey);
            if (e < 0.03 && abs(i.lnormal.z) > 0.5) {
                albedo = vec3<f32>(0.8, 0.5, 0.15);
                spec = 0.8;
                gloss = 30.0;
            } else {
                emit = i.color.rgb * i.color.w;
            }
        }
        case 9u: { // K_GLASS (moonlit window)
            let u = m.x;
            let v = m.y;
            let lead = min(abs(fract(u / 0.25) - 0.5), abs(fract(v / 0.35) - 0.5));
            let t = g.cam_pos.w;
            var c = i.color.rgb * i.color.w * (0.75 + 0.35 * fbm(vec3<f32>(u * 3.0, v * 3.0, t * 0.05)));
            c = c * smoothstep(0.02, 0.06, lead);
            return vec4<f32>(apply_fog(c, i.world), 1.0);
        }
        case 12u: { // K_PAPER (familiar)
            albedo = vec3<f32>(0.9, 0.86, 0.75);
            emit = i.color.rgb * i.color.w;
        }
        default: {
            albedo = albedo * (0.9 + 0.15 * vnoise(m * 8.0));
        }
    }
    var col = shade(albedo, n, i.world, spec, gloss, ao) + emit;
    col = apply_fog(col, i.world);
    return vec4<f32>(col, 1.0);
}

// Translucent things: slot ghosts, the veil, moonbeams. Premultiplied alpha.
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
    if (i.kind == K_SHAFT) {
        // Soft moonbeam: fades at its long ends and sides.
        let along = i.local.y + 0.5;
        let side = 1.0 - smoothstep(0.15, 0.5, abs(i.local.x));
        let side2 = 1.0 - smoothstep(0.15, 0.5, abs(i.local.z));
        let ends = smoothstep(0.0, 0.25, along) * smoothstep(1.0, 0.6, along);
        let dust = 0.75 + 0.5 * fbm(vec3<f32>(i.world.x * 2.0, i.world.y * 2.0 - t * 0.05, i.world.z * 2.0));
        let a = i.color.w * side * side2 * ends * dust;
        return vec4<f32>(i.color.rgb * a, 0.0);
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
