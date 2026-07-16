// current — curl-noise flow. The 90°-rotated gradient of fbm is a
// divergence-free velocity field (nothing pools, nothing explodes);
// luminance is streaked along it, LIC-flavored, so the whole room reads
// as slow water moving in the dark. Built to flow, incapable of spazz.
//
//   mod0 = flow (streak reach)           mod4 = bass  (surface swell)
//   mod1 = scale (eddy size)             mod7 = air   (glints)
//   mod2 = palette phase
//   mod3 = drift speed

fn cvnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u2 = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u2.x), mix(c, d, u2.x), u2.y);
}

fn cfbm(p0: vec2<f32>) -> f32 {
    var p = p0;
    var amp = 0.5;
    var acc = 0.0;
    let rot = mat2x2<f32>(0.8, 0.6, -0.6, 0.8);
    for (var i = 0; i < 3; i = i + 1) {
        acc = acc + amp * cvnoise(p);
        p = rot * p * 2.1;
        amp = amp * 0.5;
    }
    return acc;
}

fn curl(p: vec2<f32>, t: f32) -> vec2<f32> {
    let e = 0.12;
    let q = p + vec2<f32>(0.0, t);
    let dx = cfbm(q + vec2<f32>(e, 0.0)) - cfbm(q - vec2<f32>(e, 0.0));
    let dy = cfbm(q + vec2<f32>(0.0, e)) - cfbm(q - vec2<f32>(0.0, e));
    // rotate the gradient 90°: flow runs along contours, never into them
    return vec2<f32>(dy, -dx) / (2.0 * e);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let reach = mix(0.04, 0.22, clamp(u.mod0, 0.0, 1.0)) * (1.0 + u.mod4 * 0.4);
    let scale = mix(1.4, 3.6, clamp(u.mod1, 0.0, 1.0));
    let t = u.time * mix(0.01, 0.08, clamp(u.mod3, 0.0, 1.0));

    var p = in.uv * vec2<f32>(u.aspect, 1.0) * scale;
    // streak: advect along the field and gather the texture it crosses
    var acc = 0.0;
    var wsum = 0.0;
    for (var i = 0; i < 7; i = i + 1) {
        let w = 1.0 - f32(i) / 7.0;
        acc = acc + w * cfbm(p + vec2<f32>(7.3, t * 2.0));
        wsum = wsum + w;
        p = p + curl(p, t) * reach;
    }
    let streak = acc / wsum;

    // hue follows the deep field; glints ride the streak's crests
    let hue_field = cfbm(p * 0.5 + vec2<f32>(3.1, -t));
    let tone = palette(hue_field * 0.7 + u.mod2 + t * 0.35, vec3<f32>(0.0));
    var lum = streak * streak * 1.7 + 0.06;
    lum = lum + u.mod7 * 0.5 * smoothstep(0.72, 0.95, streak);
    var col = tone * lum;

    col = max(col, vec3<f32>(0.02, 0.012, 0.045));
    let r2 = dot(in.uv, in.uv);
    col = col * (1.0 - 0.24 * r2);
    return vec4<f32>(col, 1.0);
}
