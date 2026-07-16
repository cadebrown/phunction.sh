// basilica — a raymarched mandelbox: box-fold, sphere-fold, scale, repeat.
// Escher's cathedral run through a paper shredder and reassembled by a
// theorem. The camera orbits outside and dollies through the shell.
//
//   mod0 = scale (the fold's appetite)   mod4 = bass  (fold pulse)
//   mod1 = fold limit                    mod7 = air   (glow)
//   mod2 = palette phase
//   mod3 = orbit speed

fn mbox_de(p0: vec3<f32>, scale: f32, fold: f32) -> f32 {
    var z = p0;
    var dr = 1.0;
    for (var i = 0; i < 11; i = i + 1) {
        // box fold
        z = clamp(z, vec3<f32>(-fold), vec3<f32>(fold)) * 2.0 - z;
        // sphere fold
        let r2 = dot(z, z);
        if (r2 < 0.25) {
            z = z * 4.0;
            dr = dr * 4.0;
        } else if (r2 < 1.0) {
            z = z / r2;
            dr = dr / r2;
        }
        z = z * scale + p0;
        dr = dr * abs(scale) + 1.0;
    }
    return length(z) / abs(dr);
}

fn mbox_normal(p: vec3<f32>, scale: f32, fold: f32) -> vec3<f32> {
    let e = 0.002;
    return normalize(vec3<f32>(
        mbox_de(p + vec3<f32>(e, 0.0, 0.0), scale, fold) - mbox_de(p - vec3<f32>(e, 0.0, 0.0), scale, fold),
        mbox_de(p + vec3<f32>(0.0, e, 0.0), scale, fold) - mbox_de(p - vec3<f32>(0.0, e, 0.0), scale, fold),
        mbox_de(p + vec3<f32>(0.0, 0.0, e), scale, fold) - mbox_de(p - vec3<f32>(0.0, 0.0, e), scale, fold),
    ));
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // scale in the classic chaotic band; bass leans on the fold
    let scale = mix(-1.6, -2.9, clamp(u.mod0, 0.0, 1.0));
    let fold = mix(0.6, 1.25, clamp(u.mod1, 0.0, 1.0)) + u.mod4 * 0.1;
    let spin = u.time * mix(0.008, 0.12, clamp(u.mod3, 0.0, 1.0));

    // orbit camera
    let dist = 6.0;
    let eye = vec3<f32>(cos(spin) * dist, 2.2 * sin(spin * 0.6), sin(spin) * dist);
    // NB: `target` is a reserved word in WGSL — hence `aim`
    let aim = vec3<f32>(0.0);
    let fwd = normalize(aim - eye);
    let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), fwd));
    let up = cross(fwd, right);
    let uv = in.uv * vec2<f32>(u.aspect, 1.0);
    let rd = normalize(fwd * 1.6 + right * uv.x + up * uv.y);

    var total = 0.0;
    var steps = 0.0;
    var hit = false;
    // the mandelbox DE overestimates (~1.5× at range: the +p0 term inflates
    // |z|/dr), so march with heavy relaxation or rays tunnel through the set
    for (var i = 0; i < 128; i = i + 1) {
        let d = mbox_de(eye + rd * total, scale, fold);
        if (d < 0.0015 * (1.0 + total * 0.5)) {
            hit = true;
            break;
        }
        total = total + d * 0.55;
        steps = steps + 1.0;
        if (total > 20.0) { break; }
    }

    let pos = eye + rd * total;
    let cost = steps / 128.0;
    var col = vec3<f32>(0.0);
    if (hit) {
        let n = mbox_normal(pos, scale, fold);
        let key = clamp(dot(n, normalize(vec3<f32>(0.5, 0.8, -0.3))), 0.0, 1.0);
        let rim = pow(clamp(1.0 - abs(dot(n, rd)), 0.0, 1.0), 2.0);
        // orbit-trap flavored hue: radius in fold space picks the phase
        let trap = length(pos) * 0.11;
        let tone = palette(trap + u.mod2, vec3<f32>(0.0, 0.33, 0.67));
        let ao = 1.0 - cost * 0.9; // step count doubles as cheap occlusion
        col = tone * (0.15 + 0.85 * key + 0.7 * rim) * ao;
    }
    col = col + palette(u.mod2 + 0.45, vec3<f32>(0.0, 0.33, 0.67)) * cost * cost * (0.3 + u.mod7 * 1.2);
    let r2 = dot(in.uv, in.uv);
    col = col * (1.0 - 0.3 * r2);
    return vec4<f32>(col, 1.0);
}
