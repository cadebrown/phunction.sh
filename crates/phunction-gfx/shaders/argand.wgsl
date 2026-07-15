// argand — a field of rotating phasors on the complex plane.
//
// Three poles orbit the origin; the field is the product of (z - pole_k)
// phasors. Hue follows the argument of the field (the classic domain-coloring
// move), luminance follows log-magnitude contours, and the whole plane
// breathes with a slow Möbius-ish zoom. mod0 = orbit speed, mod1 = zoom,
// mod2 = contour density, mod3 = palette rotation.

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let t = u.time;
    var z = in.uv * vec2<f32>(u.aspect, 1.0);
    z = z * mix(2.2, 0.6, u.mod1);

    // Slow rotation of the whole plane.
    let spin = t * 0.05;
    z = cmul(z, vec2<f32>(cos(spin), sin(spin)));

    // Product of three orbiting simple poles.
    var f = vec2<f32>(1.0, 0.0);
    let speed = mix(0.2, 1.4, u.mod0);
    for (var k = 0; k < 3; k = k + 1) {
        let ang = t * speed * (0.3 + 0.23 * f32(k)) + TAU * f32(k) / 3.0;
        let pole = 0.85 * vec2<f32>(cos(ang), sin(ang));
        f = cmul(f, z - pole);
    }

    let arg = atan2(f.y, f.x) / TAU + 0.5;
    let mag = length(f);

    // Log-magnitude contour bands.
    let density = mix(2.0, 9.0, u.mod2);
    let band = 0.5 + 0.5 * cos(log(mag + 1e-4) * density - t * 0.7);

    var col = palette(arg + u.mod3 + t * 0.02, vec3<f32>(0.0, 0.33, 0.67));
    col = col * (0.35 + 0.65 * band);
    // Phosphor lift near the poles.
    col = col + vec3<f32>(0.10, 0.35, 0.22) * smoothstep(0.35, 0.0, mag);

    // Gentle vignette to keep projector edges clean.
    let r2 = dot(in.uv, in.uv);
    col = col * (1.0 - 0.25 * r2);

    return vec4<f32>(col, 1.0);
}
