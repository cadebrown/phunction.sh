// citadel — a kaleidoscopic IFS raymarcher: generative 3D fractal geometry.
// The space folds seven times per step (abs-fold, plane-fold, scale) and the
// result is an infinite crystalline architecture that reshapes under its
// parameters. Orbit-trap coloring on the phase wheel, glow from march cost.
//
// modulation contract (the bus drives all of it):
//   mod0 = fold scale     (structure: cathedral ↔ dust)
//   mod1 = warp           (plane-fold angle; audio RMS lands here)
//   mod2 = palette phase  (beat pulses land here)
//   mod3 = dolly          (camera distance)

const MARCH_STEPS: i32 = 72;
const FOLD_ITERS: i32 = 9;

fn rot2(p: vec2<f32>, a: f32) -> vec2<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec2<f32>(c * p.x - s * p.y, s * p.x + c * p.y);
}

// distance estimator with orbit trap
fn de(p0: vec3<f32>, scale: f32, warp: f32, trap: ptr<function, f32>) -> f32 {
    var p = p0;
    var dr = 1.0;
    *trap = 1e9;
    for (var i = 0; i < FOLD_ITERS; i = i + 1) {
        p = abs(p);
        // plane folds: sort-ish the axes so the structure stays connected
        if (p.x < p.y) { p = vec3<f32>(p.y, p.x, p.z); }
        if (p.x < p.z) { p = vec3<f32>(p.z, p.y, p.x); }
        if (p.y < p.z) { p = vec3<f32>(p.x, p.z, p.y); }
        // the warp: a slow twist that audio can push around
        let xy = rot2(p.xy, warp);
        p = vec3<f32>(xy.x, xy.y, p.z);
        p = p * scale - vec3<f32>(scale - 1.0) * vec3<f32>(1.0, 1.0, 0.5);
        if (p.z < -0.5 * (scale - 1.0)) { p.z = p.z + (scale - 1.0); }
        dr = dr * scale;
        *trap = min(*trap, length(p) * 0.25);
    }
    return length(p) / dr - 0.008;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let scale = mix(1.55, 2.9, clamp(u.mod0, 0.0, 1.0));
    let warp = (u.mod1 - 0.5) * 1.6;
    let dolly = mix(4.2, 1.6, clamp(u.mod3, 0.0, 1.0));

    // slow orbital camera; the fractal is the world, the camera is a guest
    let t = u.time * 0.11;
    let eye = vec3<f32>(cos(t) * dolly, sin(t * 0.7) * 1.2, sin(t) * dolly);
    let fwd = normalize(-eye);
    let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), fwd));
    let up = cross(fwd, right);
    let uv = in.uv * vec2<f32>(u.aspect, 1.0);
    let rd = normalize(fwd * 1.6 + right * uv.x + up * uv.y);

    var total = 0.0;
    var trap = 1e9;
    var hit = false;
    var steps = 0.0;
    for (var i = 0; i < MARCH_STEPS; i = i + 1) {
        let pos = eye + rd * total;
        var tr: f32;
        let d = de(pos, scale, warp, &tr);
        if (d < 0.0015 * total) {
            trap = tr;
            hit = true;
            break;
        }
        total = total + d;
        steps = steps + 1.0;
        if (total > 24.0) { break; }
    }

    let cost = steps / f32(MARCH_STEPS);           // march cost ≈ ambient glow
    let phase = u.mod2 + trap * 1.7 + u.time * 0.01;
    var col = vec3<f32>(0.0);
    if (hit) {
        let tone = palette(phase, vec3<f32>(0.0, 0.33, 0.67));
        let depthfade = exp(-total * 0.22);
        col = tone * (0.18 + 1.1 * depthfade) + vec3<f32>(0.9, 0.85, 1.0) * pow(1.0 - cost, 6.0) * 0.25;
    }
    // the void glows with march cost: edges of the structure catch fire
    col = col + palette(phase + 0.45, vec3<f32>(0.0, 0.33, 0.67)) * cost * cost * 0.85;

    let r2 = dot(in.uv, in.uv);
    col = col * (1.0 - 0.3 * r2);
    return vec4<f32>(col, 1.0);
}
