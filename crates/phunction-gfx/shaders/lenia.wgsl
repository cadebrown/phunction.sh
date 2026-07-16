// lenia (sim pass) — Bert Chan's continuous cellular automaton: a smooth
// generalization of Life where a bell-shaped growth function over a ring
// kernel breeds gliders, rotors, and soft blooming colonies. Verified
// natively (scratchpad lenia_tune): mu=0.26 sigma=0.045 R=6 self-sustains
// at ~35% mass over 400 steps — alive, never dying, never filling.
//
//   mod0 = growth center mu (mood)      mod4 = bass  (nutrient spark)
//   mod1 = growth width sigma           mod7 = —
//   mod2 = palette phase (present)      mod3 = sim speed

const TEXEL: vec2<f32> = vec2<f32>(1.0 / 512.0, 1.0 / 288.0);
const R: i32 = 7;
// kernel sample spacing: >1 widens the neighborhood without adding taps,
// so colonies grow to ~2.5·R·SPACING px — big soft amoebas, not grain
const SPACING: f32 = 2.6;

fn bell(x: f32, m: f32, s: f32) -> f32 {
    let d = (x - m) / s;
    return exp(-0.5 * d * d);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let uv = in.uv * 0.5 + 0.5;
    let a = textureSample(state_tex, state_samp, uv).r;

    // ring-kernel potential: weighted neighborhood, normalized on the fly
    var pot = 0.0;
    var ksum = 0.0;
    for (var dy = -R; dy <= R; dy = dy + 1) {
        for (var dx = -R; dx <= R; dx = dx + 1) {
            let d = length(vec2<f32>(f32(dx), f32(dy))) / f32(R);
            if (d <= 1.0) {
                let w = bell(d, 0.5, 0.15);
                let s = textureSample(state_tex, state_samp, uv + vec2<f32>(f32(dx), f32(dy)) * TEXEL * SPACING).r;
                pot = pot + w * s;
                ksum = ksum + w;
            }
        }
    }
    pot = pot / max(ksum, 1e-5);

    // low mu puts the system in the SPOT regime (verified natively:
    // run-length ~2 = discrete soft blobs, not the connected labyrinth
    // petri lives in) — Lenia's amoeba soup, drifting and dividing
    let mu = mix(0.14, 0.20, clamp(u.mod0, 0.0, 1.0));
    let sig = mix(0.035, 0.048, clamp(u.mod1, 0.0, 1.0));
    let dt = mix(0.05, 0.11, clamp(u.mod3, 0.0, 1.0));

    let g = 2.0 * bell(pot, mu, sig) - 1.0;
    var next = clamp(a + dt * g, 0.0, 1.0);

    // genesis: a fresh (empty) field self-seeds a low-density soup across
    // the WHOLE plane so colonies bloom everywhere on entry, never dead
    if (a < 0.001 && pot < 0.001) {
        next = hash21(floor(uv * 70.0)) * 0.5;
    }
    // bass sparks fresh life at the edges (keeps long sets evolving)
    if (u.mod4 > 0.6) {
        let cell = floor(uv * 30.0 + u.time);
        if (hash21(cell) > 0.9985) { next = 0.85; }
    }

    return vec4<f32>(next, next, next, 1.0);
}
