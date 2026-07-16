// petri (sim pass) — Gray-Scott reaction-diffusion: two chemicals in the
// R and G channels, diffusing and devouring each other into coral,
// mazes, and mitosis. The lenia-flavored one: life-like growth with
// actual memory (this fragment reads the previous frame's state).
//
//   mod0 = feed rate (sparse ↔ teeming)   mod4 = bass  (drops nutrient)
//   mod1 = kill rate (coral ↔ mitosis)
//   mod2 = palette phase (present pass)
//   mod3 = sim speed

const TEXEL: vec2<f32> = vec2<f32>(1.0 / 512.0, 1.0 / 288.0);

fn state(uv: vec2<f32>) -> vec2<f32> {
    return textureSample(state_tex, state_samp, uv).rg;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let uv = in.uv * 0.5 + 0.5;

    // nine-point Laplacian (weights 0.2 orthogonal, 0.05 diagonal)
    let c = state(uv);
    var lap = -c;
    lap = lap + 0.2 * (state(uv + vec2<f32>(TEXEL.x, 0.0)) + state(uv - vec2<f32>(TEXEL.x, 0.0)));
    lap = lap + 0.2 * (state(uv + vec2<f32>(0.0, TEXEL.y)) + state(uv - vec2<f32>(0.0, TEXEL.y)));
    lap = lap + 0.05 * (state(uv + TEXEL) + state(uv - TEXEL));
    lap = lap + 0.05 * (state(uv + vec2<f32>(TEXEL.x, -TEXEL.y)) + state(uv - vec2<f32>(TEXEL.x, -TEXEL.y)));

    let feed = mix(0.026, 0.062, clamp(u.mod0, 0.0, 1.0));
    let kill = mix(0.052, 0.066, clamp(u.mod1, 0.0, 1.0));
    let dt = mix(0.6, 1.0, clamp(u.mod3, 0.0, 1.0));

    var a = c.x;
    var b = c.y;
    let abb = a * b * b;
    a = a + (0.21 * lap.x - abb + feed * (1.0 - a)) * dt;
    b = b + (0.105 * lap.y + abb - (kill + feed) * b) * dt;

    // genesis: an EMPTY texel (fresh textures start zeroed) gets chemical
    // A and a seed pattern of B — state-based, so the dish seeds whenever
    // the mind is entered and can never fully die
    if (c.x < 0.01 && c.y < 0.01) {
        a = 1.0;
        let d = distance(uv, vec2<f32>(0.5));
        b = select(0.0, 1.0, abs(d - 0.15) < 0.02 || hash21(floor(uv * 40.0)) > 0.97);
    }
    if (u.mod4 > 0.55) {
        let cell = floor(uv * 24.0 + u.time);
        if (hash21(cell) > 0.9985) {
            b = 0.9;
        }
    }

    return vec4<f32>(clamp(a, 0.0, 1.0), clamp(b, 0.0, 1.0), 0.0, 1.0);
}
