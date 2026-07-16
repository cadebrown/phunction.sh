// gyroid — a raymarched triply-periodic minimal surface, twisted.
// The gyroid divides space into two congruent labyrinths; we fly down one
// channel while the lattice breathes. Weird geometry as promised: no
// vertices, no faces, just an implicit surface that fills the universe.
//
// The flight path hugs a channel center (g = +1 at x ≈ π/2, y ≈ 0) and the
// twist is bounded (sin of z, not z itself) so the channel never shears away
// from under the camera. Marching uses |d| so even a wall clip recovers.
//
//   mod0 = thickness (lace ↔ bone)      mod4 = bass    (cell pulse)
//   mod1 = twist                        mod7 = air     (glow)
//   mod2 = palette phase
//   mod3 = flight speed

fn rot2g(p: vec2<f32>, a: f32) -> vec2<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec2<f32>(c * p.x - s * p.y, s * p.x + c * p.y);
}

// Field frequency: the raw gyroid's period is 2π, so at flight distance a
// single cell wall fills the whole view and reads as a smooth blob. Scale
// space up so several cells fit the frustum and the lattice is legible.
const GFREQ: f32 = 2.2;

fn gyroid_de(p0: vec3<f32>, thick: f32, twist: f32) -> f32 {
    var p = p0 * GFREQ;
    // bounded shear: space breathes around the flight axis instead of
    // winding up forever
    let xy = rot2g(p.xy, sin(p.z * 0.35) * twist);
    p = vec3<f32>(xy.x, xy.y, p.z);
    let g = dot(sin(p), cos(p.yzx));
    return (abs(g) - thick) * 0.4 / GFREQ;
}

fn gyroid_normal(p: vec3<f32>, thick: f32, twist: f32) -> vec3<f32> {
    let e = 0.015;
    return normalize(vec3<f32>(
        gyroid_de(p + vec3<f32>(e, 0.0, 0.0), thick, twist)
            - gyroid_de(p - vec3<f32>(e, 0.0, 0.0), thick, twist),
        gyroid_de(p + vec3<f32>(0.0, e, 0.0), thick, twist)
            - gyroid_de(p - vec3<f32>(0.0, e, 0.0), thick, twist),
        gyroid_de(p + vec3<f32>(0.0, 0.0, e), thick, twist)
            - gyroid_de(p - vec3<f32>(0.0, 0.0, e), thick, twist),
    ));
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // stay lace-like: the channel (|g| ≈ 1) must remain open space
    let thick = mix(0.04, 0.30, clamp(u.mod0, 0.0, 1.0)) + u.mod4 * 0.12;
    let twist = (u.mod1 - 0.5) * 2.4;
    let speed = mix(0.08, 0.9, clamp(u.mod3, 0.0, 1.0));

    // fly down the labyrinth's throat, hugging the channel center
    // (g = +1 at scaled x = π/2, so world x = π/2 / GFREQ) — small wobble,
    // wide-ish FOV, so several cells stay in frame instead of one wall
    let t = u.time * speed;
    let eye = vec3<f32>(
        1.5708 / GFREQ + sin(t * 0.31) * 0.12,
        cos(t * 0.23) * 0.12,
        t * 0.55,
    );
    let uv = in.uv * vec2<f32>(u.aspect, 1.0);
    let fwd = normalize(vec3<f32>(sin(t * 0.13) * 0.3, cos(t * 0.11) * 0.2, 1.0));
    let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), fwd));
    let up = cross(fwd, right);
    let rd = normalize(fwd * 1.1 + right * uv.x + up * uv.y);

    var total = 0.0;
    var steps = 0.0;
    var hit = false;
    for (var i = 0; i < 110; i = i + 1) {
        let d = abs(gyroid_de(eye + rd * total, thick, twist));
        if (d < 0.002 * (1.0 + total * 0.5)) {
            hit = true;
            break;
        }
        total = total + d * 0.9;
        steps = steps + 1.0;
        if (total > 14.0) { break; }
    }

    let pos = eye + rd * total;
    let cost = steps / 110.0;
    let q = pos * GFREQ;
    // phase coloring in *scaled* space so each cell gets its own hue band
    let cell = fract((q.x + q.y + q.z) * 0.11 + u.mod2 + u.time * 0.008);
    var col = vec3<f32>(0.0);
    if (hit) {
        let n = gyroid_normal(pos, thick, twist);
        let key = clamp(dot(n, normalize(vec3<f32>(0.6, 0.7, -0.4))), 0.0, 1.0);
        let rim = pow(1.0 - abs(dot(n, rd)), 2.0);
        // the conjugate gyroid stripes the surface — lattice made legible
        let g2 = dot(sin(q.yzx), cos(q.zxy));
        let stripe = 0.55 + 0.45 * smoothstep(0.05, 0.35, abs(fract(g2 * 1.5) - 0.5));
        let tone = palette(cell, vec3<f32>(0.0, 0.33, 0.67));
        let fog = exp(-total * 0.10);
        col = tone * (0.18 + 0.9 * key + 0.8 * rim) * stripe * (0.2 + fog);
    }
    // quiet glow for escaped/grazing rays — depth cue, not a wash
    col = col + palette(cell + 0.4, vec3<f32>(0.0, 0.33, 0.67)) * cost * cost * (0.25 + u.mod7 * 0.9);
    let r2 = dot(in.uv, in.uv);
    col = col * (1.0 - 0.28 * r2);
    return vec4<f32>(col, 1.0);
}
