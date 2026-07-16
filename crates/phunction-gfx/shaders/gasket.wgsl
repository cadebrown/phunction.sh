// gasket — an apollonian packing (after iq's construction): repeated
// inversion in fract-space packs spheres into spheres into spheres. We
// drift through the packing while the inversion ratio breathes.
//
//   mod0 = inversion ratio               mod4 = bass  (ratio pulse)
//   mod1 = zoom                          mod7 = air   (glow)
//   mod2 = palette phase
//   mod3 = drift speed

fn gasket_de(p0: vec3<f32>, ratio: f32) -> vec2<f32> {
    var p = p0;
    var scale = 1.0;
    var trap = 1e9;
    for (var i = 0; i < 9; i = i + 1) {
        p = -1.0 + 2.0 * fract(0.5 * p + 0.5);
        let r2 = dot(p, p);
        trap = min(trap, r2);
        let k = ratio / r2;
        p = p * k;
        scale = scale * k;
    }
    // distance to the packing's y-plane slices; trap colors the spheres
    let d = 0.25 * abs(p.y) / scale;
    return vec2<f32>(d, trap);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let ratio = mix(1.0, 1.22, clamp(u.mod0, 0.0, 1.0)) + u.mod4 * 0.05;
    let zoom = mix(1.8, 0.7, clamp(u.mod1, 0.0, 1.0));
    let t = u.time * mix(0.008, 0.12, clamp(u.mod3, 0.0, 1.0));

    // drift through the packing
    let eye = vec3<f32>(t * 0.4, 0.12 * sin(t * 0.7), t * 0.23);
    let uv = in.uv * vec2<f32>(u.aspect, 1.0);
    let fwd = normalize(vec3<f32>(0.42 + 0.2 * sin(t * 0.31), 0.18 * cos(t * 0.23), 1.0));
    let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), fwd));
    let up = cross(fwd, right);
    let rd = normalize(fwd * zoom + right * uv.x + up * uv.y);

    var total = 0.0;
    var steps = 0.0;
    var hit = false;
    var trap = 0.0;
    for (var i = 0; i < 90; i = i + 1) {
        let dt = gasket_de(eye + rd * total, ratio);
        if (dt.x < 0.0012 * (1.0 + total)) {
            hit = true;
            trap = dt.y;
            break;
        }
        total = total + dt.x * 0.9;
        steps = steps + 1.0;
        if (total > 12.0) { break; }
    }

    let cost = steps / 90.0;
    var col = vec3<f32>(0.0);
    if (hit) {
        // the orbit trap picks each sphere's phase on the wheel; the step
        // count doubles as occlusion in the packing's crevices
        let tone = palette(sqrt(trap) * 1.4 + u.mod2, vec3<f32>(0.0, 0.33, 0.67));
        let fog = exp(-total * 0.35);
        let ao = 1.0 - cost * 0.85;
        col = tone * (0.2 + 1.3 * fog) * ao;
    }
    col = col + palette(u.mod2 + 0.4, vec3<f32>(0.0, 0.33, 0.67)) * cost * cost * (0.35 + u.mod7 * 1.3);
    let r2 = dot(in.uv, in.uv);
    col = col * (1.0 - 0.3 * r2);
    return vec4<f32>(col, 1.0);
}
