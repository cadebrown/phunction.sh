// Shared by every shader phunctor: fullscreen triangle + uniforms + helpers.
// Rust-side layout: shader_phunctor.rs `struct Uniforms` — keep in sync.

struct U {
    time: f32,
    aspect: f32,
    mod0: f32,
    mod1: f32,
    mod2: f32,
    mod3: f32,
    _pad0: f32,
    _pad1: f32,
}
@group(0) @binding(0) var<uniform> u: U;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) i: u32) -> VsOut {
    // One triangle that covers the screen; uv in [-1, 1] with aspect applied
    // in the fragment shaders (x widened by u.aspect).
    var out: VsOut;
    let x = f32(i32(i) - 1) * 3.0;
    let y = f32(i32(i & 1u) * 2 - 1) * 3.0;
    out.pos = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

// ---- helpers ----

const TAU: f32 = 6.28318530718;

// Complex multiply.
fn cmul(a: vec2<f32>, b: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(a.x * b.x - a.y * b.y, a.x * b.y + a.y * b.x);
}

// IQ's cosine palette — the psychedelic workhorse.
fn palette(t: f32, phase: vec3<f32>) -> vec3<f32> {
    return 0.5 + 0.5 * cos(TAU * (vec3<f32>(t) + phase));
}

// Cheap value-noise-ish hash.
fn hash21(p: vec2<f32>) -> f32 {
    var q = fract(p * vec2<f32>(123.34, 456.21));
    q = q + dot(q, q + 45.32);
    return fract(q.x * q.y);
}
