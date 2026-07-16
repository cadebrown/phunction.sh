// maw — a pseudo-Kleinian cavity (Knighty's fold): box-fold + sphere
// inversion, ten deep. The camera glides INSIDE the set, through endless
// alien vaulting — arches inside arches, a cathedral with no outside.
// DE verified natively (scratchpad de_check): 5/5 rays hit in ≤73 steps.
//
//   mod0 = cell size (vault span)       mod4 = bass  (walls breathe)
//   mod1 = ceiling bias                 mod7 = air   (fog glow)
//   mod2 = palette phase
//   mod3 = glide speed

fn maw_de(p0: vec3<f32>, csize: vec3<f32>) -> vec2<f32> {
    var p = p0;
    var scale = 1.0;
    var trap = 1e9;
    for (var i = 0; i < 10; i = i + 1) {
        p = 2.0 * clamp(p, -csize, csize) - p;
        let r2 = dot(p, p);
        let k = max(1.0 / max(r2, 1e-9), 1.0);
        p = p * k;
        scale = scale * k;
        trap = min(trap, r2);
    }
    let l = length(p.xy);
    let rxy = max(l - 4.0, -(l * p.z) / 4.0);
    return vec2<f32>(rxy / abs(scale), trap);
}

fn maw_normal(p: vec3<f32>, csize: vec3<f32>) -> vec3<f32> {
    let e = 0.001;
    return normalize(vec3<f32>(
        maw_de(p + vec3<f32>(e, 0.0, 0.0), csize).x - maw_de(p - vec3<f32>(e, 0.0, 0.0), csize).x,
        maw_de(p + vec3<f32>(0.0, e, 0.0), csize).x - maw_de(p - vec3<f32>(0.0, e, 0.0), csize).x,
        maw_de(p + vec3<f32>(0.0, 0.0, e), csize).x - maw_de(p - vec3<f32>(0.0, 0.0, e), csize).x,
    ));
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // fold cell: mod0 widens the vaults, mod1 flattens the ceilings,
    // bass leans on the walls (breathes with the drone)
    let cs = mix(0.85, 1.05, clamp(u.mod0, 0.0, 1.0)) + u.mod4 * 0.03;
    let cy = mix(0.8, 1.0, clamp(u.mod1, 0.0, 1.0));
    let csize = vec3<f32>(cs * 0.92436, cy * 0.90756, cs * 0.92436);

    // glide: a closed loop through the cavity — bounded forever, so a
    // session hours deep still sits inside the set (and inside f32)
    let t = u.time * mix(0.01, 0.09, clamp(u.mod3, 0.0, 1.0));
    let eye = vec3<f32>(
        0.12 * sin(t * 0.7),
        0.55 + 0.1 * sin(t * 0.43),
        -1.2 + 1.8 * sin(t * 0.17),
    );
    let fwd = normalize(vec3<f32>(0.22 * sin(t * 0.5), 0.08 * cos(t * 0.31), 1.0));
    let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), fwd));
    let up = cross(fwd, right);
    let uv = in.uv * vec2<f32>(u.aspect, 1.0);
    let rd = normalize(fwd * 1.35 + right * uv.x + up * uv.y);

    var total = 0.0;
    var steps = 0.0;
    var hit = false;
    var trap = 0.0;
    for (var i = 0; i < 140; i = i + 1) {
        let dt = maw_de(eye + rd * total, csize);
        if (dt.x < 0.0008 * (1.0 + total * 0.6)) {
            hit = true;
            trap = dt.y;
            break;
        }
        total = total + dt.x * 0.7;
        steps = steps + 1.0;
        if (total > 14.0) { break; }
    }

    let pos = eye + rd * total;
    let cost = steps / 140.0;
    var col = vec3<f32>(0.0);
    if (hit) {
        let n = maw_normal(pos, csize);
        let key = clamp(dot(n, normalize(vec3<f32>(0.4, 0.75, -0.5))), 0.0, 1.0);
        let rim = pow(clamp(1.0 - abs(dot(n, rd)), 0.0, 1.0), 2.5);
        // the orbit trap (closest inversion radius) paints the vaulting
        let tone = palette(sqrt(trap) * 0.6 + u.mod2, vec3<f32>(0.0, 0.33, 0.67));
        let ao = 1.0 - cost * 0.7;
        col = tone * (0.3 + 1.2 * key + 1.0 * rim) * ao * 1.6;
    }
    // depth fog in canon air — the cavity recedes into indigo
    let fog = 1.0 - exp(-total * 0.3);
    let air = palette(u.mod2 + 0.5, vec3<f32>(0.0, 0.33, 0.67));
    col = mix(col, air * 0.3, fog);
    col = col + air * cost * cost * (0.25 + u.mod7 * 1.1);
    let r2 = dot(in.uv, in.uv);
    col = col * (1.0 - 0.3 * r2);
    return vec4<f32>(col, 1.0);
}
