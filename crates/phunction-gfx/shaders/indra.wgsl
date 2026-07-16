// indra — a Kleinian limit set on the Maskit slice (Jos Leys's fast
// algorithm, verified natively in scratchpad vis_check: thin set, 4% of
// the strip at the canonical point). Two Möbius transformations weave a
// filigree of tangent circles; the parameter t = a+bi walks the boundary
// of Maskit space, so the lace itself slowly rewires.
//
//   mod0 = knot (Re t)                  mod4 = bass  (glow swell)
//   mod1 = weave (Im t)                 mod7 = air
//   mod2 = palette phase
//   mod3 = drift speed

fn kleinian_de(z0: vec2<f32>, a: f32, b: f32) -> vec2<f32> {
    var z = z0;
    var lz = z + vec2<f32>(1.0, 0.0);
    var llz = z - vec2<f32>(1.0, 0.0);
    var df = 1.0;
    var trap = 1e9;
    let f = sign(b);
    for (var i = 0; i < 80; i = i + 1) {
        // wrap x into the fundamental strip, sheared along the lattice
        z.x = z.x + f * b / a * z.y;
        z.x = fract((z.x + 1.0) * 0.5) * 2.0 - 1.0;
        z.x = z.x - f * b / a * z.y;
        // fold across Leys's wobbled separating line
        if (z.y >= a * (0.5 + 0.4 * sin(f * 3.14159265 * (z.x + b * 0.5) / a))) {
            z = vec2<f32>(-b, a) - z;
        }
        // TransA: unit-circle inversion + translate (the Möbius a)
        let ir = 1.0 / max(dot(z, z), 1e-12);
        z = z * (-ir) + vec2<f32>(-b, a);
        df = df * ir;
        trap = min(trap, dot(z, z));
        if (dot(z - llz, z - llz) < 1e-10) { break; }
        llz = lz;
        lz = z;
        if (z.y < -0.1 || z.y > a + 0.1) { break; }
    }
    let y = min(z.y, a - z.y);
    return vec2<f32>(y / max(df, 2.0), trap);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // the Maskit parameter breathes: mods set the region, time walks it
    let t = u.time * mix(0.005, 0.05, clamp(u.mod3, 0.0, 1.0));
    let a = mix(1.90, 1.975, clamp(u.mod0, 0.0, 1.0)) + 0.012 * sin(t * 0.61);
    let b = mix(0.004, 0.05, clamp(u.mod1, 0.0, 1.0)) + 0.006 * sin(t * 0.83);

    // the necklace runs along the strip's long axis; on a wide screen we
    // lay that axis HORIZONTAL and show ~1.5 periods, slowly panning
    let zoom = 0.95 + 0.1 * sin(t * 0.37);
    let q = in.uv * vec2<f32>(u.aspect, 1.0) * zoom;
    var p = vec2<f32>(q.y, q.x);       // rotate 90°: necklace runs horizontal
    p.x = p.x + 0.4 * sin(t * 0.19);   // pan along the necklace
    p.y = p.y + a * 0.5;               // center the strip

    let dt = kleinian_de(p, a, b);
    let de = abs(dt.x);

    // the filigree: a bright luminous line, a mid glow, and a wide haze so
    // the lace lifts off the black instead of being a hairline
    let core = 0.0022 / (de + 0.0022);
    let glow = 0.02 / (de * 9.0 + 0.02);
    let haze = 0.12 / (de * 40.0 + 0.12);
    let tone = palette(sqrt(dt.y) * 0.2 + u.mod2 + p.x * 0.1, vec3<f32>(0.0, 0.33, 0.67));
    let air = palette(u.mod2 + 0.5, vec3<f32>(0.0, 0.33, 0.67));
    var col = tone * core * (1.2 + u.mod4 * 1.5);
    col = col + tone * glow * glow * 0.8;
    col = col + air * haze * haze * 0.5;
    col = col + air * 0.03 * (1.0 + u.mod7);
    let r2 = dot(in.uv, in.uv);
    col = col * (1.0 - 0.28 * r2);
    return vec4<f32>(col, 1.0);
}
