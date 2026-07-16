// bulb — the Mandelbulb, power morphing live. Not the postcard: the power
// slides 6→10 with mod0 so the set continuously grows and sheds lobes,
// and the orbit trap drives the palette so the surface is veined, not flat.
// DE verified natively (scratchpad de_check): 8/8 rays hit in ≤50 steps.
//
//   mod0 = power morph (6..10)          mod4 = bass  (power pulse)
//   mod1 = surface detail bias          mod7 = air   (glow)
//   mod2 = palette phase
//   mod3 = orbit speed

fn bulb_de(p0: vec3<f32>, power: f32) -> vec2<f32> {
    var z = p0;
    var dr = 1.0;
    var r = 0.0;
    var trap = 1e9;
    for (var i = 0; i < 12; i = i + 1) {
        r = length(z);
        if (r > 2.0) { break; }
        trap = min(trap, r);
        let theta = acos(clamp(z.z / max(r, 1e-9), -1.0, 1.0)) * power;
        let phi = atan2(z.y, z.x) * power;
        dr = pow(r, power - 1.0) * power * dr + 1.0;
        let zr = pow(r, power);
        z = zr * vec3<f32>(sin(theta) * cos(phi), sin(theta) * sin(phi), cos(theta)) + p0;
    }
    return vec2<f32>(0.5 * log(max(r, 1e-9)) * r / dr, trap);
}

fn bulb_normal(p: vec3<f32>, power: f32) -> vec3<f32> {
    let e = 0.0015;
    return normalize(vec3<f32>(
        bulb_de(p + vec3<f32>(e, 0.0, 0.0), power).x - bulb_de(p - vec3<f32>(e, 0.0, 0.0), power).x,
        bulb_de(p + vec3<f32>(0.0, e, 0.0), power).x - bulb_de(p - vec3<f32>(0.0, e, 0.0), power).x,
        bulb_de(p + vec3<f32>(0.0, 0.0, e), power).x - bulb_de(p - vec3<f32>(0.0, 0.0, e), power).x,
    ));
}

// cheap soft shadow: a few DE probes toward the key light
fn bulb_shadow(p: vec3<f32>, l: vec3<f32>, power: f32) -> f32 {
    var t = 0.02;
    var sh = 1.0;
    for (var i = 0; i < 24; i = i + 1) {
        let d = bulb_de(p + l * t, power).x;
        sh = min(sh, 9.0 * d / t);
        t = t + clamp(d, 0.01, 0.12);
        if (sh < 0.02 || t > 2.2) { break; }
    }
    return clamp(sh, 0.0, 1.0);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // the power breathes: mod0 sets the register, bass nudges it
    let power = mix(6.0, 10.0, clamp(u.mod0, 0.0, 1.0)) + u.mod4 * 0.5;
    let spin = u.time * mix(0.008, 0.1, clamp(u.mod3, 0.0, 1.0));

    let dist = 2.5;
    let eye = vec3<f32>(cos(spin) * dist, 1.1 * sin(spin * 0.43), sin(spin) * dist);
    let aim = vec3<f32>(0.0);
    let fwd = normalize(aim - eye);
    let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), fwd));
    let up = cross(fwd, right);
    let uv = in.uv * vec2<f32>(u.aspect, 1.0);
    let rd = normalize(fwd * 1.7 + right * uv.x + up * uv.y);

    let eps = mix(0.0012, 0.0004, clamp(u.mod1, 0.0, 1.0));
    var total = 0.0;
    var steps = 0.0;
    var hit = false;
    var trap = 0.0;
    for (var i = 0; i < 128; i = i + 1) {
        let dt = bulb_de(eye + rd * total, power);
        if (dt.x < eps * (1.0 + total)) {
            hit = true;
            trap = dt.y;
            break;
        }
        total = total + dt.x * 0.9;
        steps = steps + 1.0;
        if (total > 8.0) { break; }
    }

    let cost = steps / 128.0;
    var col = vec3<f32>(0.0);
    if (hit) {
        let pos = eye + rd * total;
        let n = bulb_normal(pos, power);
        let ldir = normalize(vec3<f32>(0.55, 0.7, -0.45));
        let key = clamp(dot(n, ldir), 0.0, 1.0) * mix(0.3, 1.0, bulb_shadow(pos, ldir, power));
        let rim = pow(clamp(1.0 - abs(dot(n, rd)), 0.0, 1.0), 2.0);
        // veins: the orbit trap phases the palette across the lobes
        let tone = palette(trap * 0.9 + u.mod2, vec3<f32>(0.0, 0.33, 0.67));
        let ao = 1.0 - cost * 0.75;
        col = tone * (0.28 + 1.25 * key + 0.9 * rim) * ao * 1.5;
    }
    let air = palette(u.mod2 + 0.45, vec3<f32>(0.0, 0.33, 0.67));
    // never a void: faint canon air behind the set
    col = col + air * (0.05 + cost * cost * (0.35 + u.mod7 * 1.2));
    let r2 = dot(in.uv, in.uv);
    col = col * (1.0 - 0.3 * r2);
    return vec4<f32>(col, 1.0);
}
