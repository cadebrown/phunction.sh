// silk — domain-warped flow noise: fbm(p + fbm(p + fbm(p))), everything
// slow. No marching, no edges — just fabric folding through the phase
// wheel. The flowy one: built to breathe, incapable of spazzing.
//
//   mod0 = depth (how far the warp folds)   mod4 = bass  (gentle swell)
//   mod1 = grain (weave frequency)          mod7 = air   (sheen)
//   mod2 = palette phase
//   mod3 = drift speed

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u2 = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u2.x), mix(c, d, u2.x), u2.y);
}

fn fbm(p0: vec2<f32>) -> f32 {
    var p = p0;
    var amp = 0.5;
    var acc = 0.0;
    // the classic octave rotation keeps axes from ghosting through
    let rot = mat2x2<f32>(0.8, 0.6, -0.6, 0.8);
    for (var i = 0; i < 5; i = i + 1) {
        acc = acc + amp * vnoise(p);
        p = rot * p * 2.0;
        amp = amp * 0.5;
    }
    return acc;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let depth = mix(1.2, 4.5, clamp(u.mod0, 0.0, 1.0));
    let grain = mix(1.1, 3.2, clamp(u.mod1, 0.0, 1.0));
    let t = u.time * mix(0.01, 0.09, clamp(u.mod3, 0.0, 1.0));
    let swell = 1.0 + u.mod4 * 0.12;

    let p = in.uv * vec2<f32>(u.aspect, 1.0) * grain * swell;

    // three folds of the plane into itself (iq's warp), each drifting on
    // its own slow clock
    let q = vec2<f32>(
        fbm(p + vec2<f32>(0.0, t)),
        fbm(p + vec2<f32>(5.2, -t * 1.31)),
    );
    let r = vec2<f32>(
        fbm(p + depth * q + vec2<f32>(1.7, 9.2) + t * 0.35),
        fbm(p + depth * q + vec2<f32>(8.3, 2.8) - t * 0.21),
    );
    let f = fbm(p + depth * r);

    // hue rides the deepest fold; luminance is shaped by the middle one,
    // so the sheets read as lit fabric instead of fog
    let tone = palette(
        f * 0.9 + q.x * 0.35 + u.mod2 + t * 0.4,
        vec3<f32>(0.0, 0.33, 0.67),
    );
    var lum = f * f * 1.6 + r.y * 0.35 + 0.08;
    lum = lum * (0.75 + u.mod7 * 0.6 * q.y);
    var col = tone * lum;

    // deep-violet floor so the darks stay in canon, never dead black
    col = max(col, vec3<f32>(0.03, 0.015, 0.05));
    let r2 = dot(in.uv, in.uv);
    col = col * (1.0 - 0.22 * r2);
    return vec4<f32>(col, 1.0);
}
