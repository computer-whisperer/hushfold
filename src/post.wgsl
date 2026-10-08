// Bloom chain + tonemap.

struct PostParams {
    exposure: f32,
    bloom: f32,
    time: f32,
    vignette: f32,
};

@group(0) @binding(0) var src: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var bloom_tex: texture_2d<f32>;
@group(0) @binding(3) var<uniform> pp: PostParams;

struct FOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_full(@builtin(vertex_index) vi: u32) -> FOut {
    var o: FOut;
    let x = f32((vi << 1u) & 2u);
    let y = f32(vi & 2u);
    o.pos = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
    o.uv = vec2<f32>(x, y);
    return o;
}

fn down13(uv: vec2<f32>) -> vec3<f32> {
    let ts = 1.0 / vec2<f32>(textureDimensions(src));
    let a = textureSample(src, samp, uv + ts * vec2<f32>(-2.0, -2.0)).rgb;
    let b = textureSample(src, samp, uv + ts * vec2<f32>(0.0, -2.0)).rgb;
    let c = textureSample(src, samp, uv + ts * vec2<f32>(2.0, -2.0)).rgb;
    let d = textureSample(src, samp, uv + ts * vec2<f32>(-2.0, 0.0)).rgb;
    let e = textureSample(src, samp, uv).rgb;
    let f = textureSample(src, samp, uv + ts * vec2<f32>(2.0, 0.0)).rgb;
    let gg = textureSample(src, samp, uv + ts * vec2<f32>(-2.0, 2.0)).rgb;
    let h = textureSample(src, samp, uv + ts * vec2<f32>(0.0, 2.0)).rgb;
    let i = textureSample(src, samp, uv + ts * vec2<f32>(2.0, 2.0)).rgb;
    let j = textureSample(src, samp, uv + ts * vec2<f32>(-1.0, -1.0)).rgb;
    let k = textureSample(src, samp, uv + ts * vec2<f32>(1.0, -1.0)).rgb;
    let l = textureSample(src, samp, uv + ts * vec2<f32>(-1.0, 1.0)).rgb;
    let m = textureSample(src, samp, uv + ts * vec2<f32>(1.0, 1.0)).rgb;
    return e * 0.125 + (a + c + gg + i) * 0.03125 + (b + d + f + h) * 0.0625 + (j + k + l + m) * 0.125;
}

@fragment
fn fs_down_first(i: FOut) -> @location(0) vec4<f32> {
    let c = min(down13(i.uv), vec3<f32>(60.0));
    let lum = max(max(c.r, c.g), c.b);
    let knee = 0.6;
    let thr = 1.0;
    var soft = clamp(lum - thr + knee, 0.0, 2.0 * knee);
    soft = soft * soft / (4.0 * knee + 1e-4);
    let w = max(soft, lum - thr) / max(lum, 1e-4);
    return vec4<f32>(c * w, 1.0);
}

@fragment
fn fs_down(i: FOut) -> @location(0) vec4<f32> {
    return vec4<f32>(down13(i.uv), 1.0);
}

@fragment
fn fs_up(i: FOut) -> @location(0) vec4<f32> {
    let ts = 1.0 / vec2<f32>(textureDimensions(src));
    var c = textureSample(src, samp, i.uv).rgb * 4.0;
    c += textureSample(src, samp, i.uv + ts * vec2<f32>(-1.0, 0.0)).rgb * 2.0;
    c += textureSample(src, samp, i.uv + ts * vec2<f32>(1.0, 0.0)).rgb * 2.0;
    c += textureSample(src, samp, i.uv + ts * vec2<f32>(0.0, -1.0)).rgb * 2.0;
    c += textureSample(src, samp, i.uv + ts * vec2<f32>(0.0, 1.0)).rgb * 2.0;
    c += textureSample(src, samp, i.uv + ts * vec2<f32>(-1.0, -1.0)).rgb;
    c += textureSample(src, samp, i.uv + ts * vec2<f32>(1.0, -1.0)).rgb;
    c += textureSample(src, samp, i.uv + ts * vec2<f32>(-1.0, 1.0)).rgb;
    c += textureSample(src, samp, i.uv + ts * vec2<f32>(1.0, 1.0)).rgb;
    return vec4<f32>(c / 16.0, 1.0);
}

fn aces(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_tonemap(i: FOut) -> @location(0) vec4<f32> {
    let hdr = textureSample(src, samp, i.uv).rgb;
    let bl = textureSample(bloom_tex, samp, i.uv).rgb;
    var c = (hdr + bl * pp.bloom) * pp.exposure;
    // Gentle warm grade.
    c = c * vec3<f32>(1.04, 1.0, 0.94);
    c = aces(c);
    let d = i.uv - vec2<f32>(0.5);
    let vig = 1.0 - pp.vignette * dot(d, d) * 1.6;
    c = c * vig;
    // Encode for a UNORM swapchain, then dither to hide banding in dark gradients.
    c = select(1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055, c * 12.92, c <= vec3<f32>(0.0031308));
    let n = fract(sin(dot(i.pos.xy + fract(pp.time) * 17.0, vec2<f32>(12.9898, 78.233))) * 43758.5453);
    c = c + (n - 0.5) / 255.0;
    return vec4<f32>(clamp(c, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
