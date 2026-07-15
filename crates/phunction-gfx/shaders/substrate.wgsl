// substrate — the living background. A domain-colored field of three slow
// drifting poles, dimmed and mixed toward the plane color so content stays
// legible on top. This is the old site's Substrate reborn in wgpu: the whole
// site floats on a holomorphic function. Tuned soft: it should read as
// weather, not as a poster.

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let t = u.time * 0.35;
    var z = in.uv * vec2<f32>(u.aspect, 1.0) * 1.35;

    // the whole plane turns, imperceptibly
    let spin = t * 0.04;
    z = cmul(z, vec2<f32>(cos(spin), sin(spin)));

    // three poles wandering on breathing orbits
    var f = vec2<f32>(0.35, 0.0);
    for (var k = 0; k < 3; k = k + 1) {
        let ang = t * (0.11 + 0.07 * f32(k)) + TAU * f32(k) / 3.0;
        let r = 0.9 + 0.35 * sin(t * 0.13 + f32(k) * 2.1);
        let pole = r * vec2<f32>(cos(ang), sin(ang));
        f = cmul(f, z - pole);
    }

    let arg = atan2(f.y, f.x) / TAU + 0.5;
    let mag = length(f);
    // broad, soft magnitude bands — structure without stripes
    let band = 0.5 + 0.5 * cos(log(mag + 1e-4) * 2.2 - t * 0.5);

    var col = palette(arg + t * 0.015, vec3<f32>(0.0, 0.33, 0.67));
    col = col * (0.22 + 0.30 * band);

    // sink toward the plane so ink stays readable; mod2 = intensity
    let plane = vec3<f32>(0.055, 0.045, 0.10);
    col = mix(plane, col, clamp(u.mod2 + 0.1, 0.0, 1.0));

    let r2 = dot(in.uv, in.uv);
    col = col * (1.0 - 0.35 * r2);
    return vec4<f32>(col, 1.0);
}
