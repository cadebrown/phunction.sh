// cortex — a neural field: a genuine neural network (a compositional
// pattern-producing network, 5→12→12→3, tanh activations) evaluated per
// pixel, per frame, locally on your GPU. The weights are procedural —
// hashed from a seed you can scrub — so every seed is a different mind.
// Inputs: position, radius, and two oscillators; the audio bands bias the
// hidden layer, so the network literally *listens*.
//
//   mod0 = seed (scrub through minds)   mod4..7 = spectrum bands (bias)
//   mod1 = zoom
//   mod2 = palette phase
//   mod3 = time flow

fn whash(seed: f32, a: f32, b: f32) -> f32 {
    // deterministic weight in [-1, 1]
    return fract(sin(seed * 127.1 + a * 311.7 + b * 74.7) * 43758.5453) * 2.0 - 1.0;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let seed = floor(u.mod0 * 32.0) + 7.0;
    let zoom = mix(2.4, 0.7, clamp(u.mod1, 0.0, 1.0));
    let tf = u.time * mix(0.02, 0.25, clamp(u.mod3, 0.0, 1.0));

    let p = in.uv * vec2<f32>(u.aspect, 1.0) * zoom;
    // network inputs: position, radius, and *oscillating* coordinates — the
    // sin inputs are what give a CPPN its bands and filaments; without them
    // (and without enough gain to saturate the tanh) the field collapses
    // into a linear gradient
    let x0 = array<f32, 5>(
        p.x,
        p.y,
        length(p) * 1.4,
        sin(p.x * 2.7 + tf),
        sin(p.y * 2.3 - tf * 0.8),
    );

    // layer 1: 5 → 12
    var h1: array<f32, 12>;
    for (var j = 0; j < 12; j = j + 1) {
        var acc = whash(seed, f32(j), 99.0); // bias
        for (var i = 0; i < 5; i = i + 1) {
            acc = acc + x0[i] * whash(seed, f32(j), f32(i)) * 2.4;
        }
        // the audio reaches into the hidden layer
        acc = acc + u.mod4 * whash(seed, f32(j), 201.0)
                  + u.mod5 * whash(seed, f32(j), 202.0)
                  + u.mod6 * whash(seed, f32(j), 203.0)
                  + u.mod7 * whash(seed, f32(j), 204.0);
        h1[j] = tanh(acc);
    }

    // layer 2: 12 → 12
    var h2: array<f32, 12>;
    for (var j = 0; j < 12; j = j + 1) {
        var acc = whash(seed + 31.0, f32(j), 99.0);
        for (var i = 0; i < 12; i = i + 1) {
            acc = acc + h1[i] * whash(seed + 31.0, f32(j), f32(i)) * 1.8;
        }
        h2[j] = tanh(acc);
    }

    // output layer: 12 → 3, mapped through the phase palette
    var o: array<f32, 3>;
    for (var j = 0; j < 3; j = j + 1) {
        var acc = whash(seed + 63.0, f32(j), 99.0);
        for (var i = 0; i < 12; i = i + 1) {
            acc = acc + h2[i] * whash(seed + 63.0, f32(j), f32(i));
        }
        o[j] = tanh(acc) * 0.5 + 0.5;
    }

    // o[0] picks the hue on the wheel, o[1] carves luminance bands,
    // o[2] adds a counter-phase wash — the network paints in our canon
    var col = palette(o[0] + u.mod2, vec3<f32>(0.0, 0.33, 0.67)) * (0.25 + 0.75 * o[1]);
    col = col + palette(o[0] + 0.5 + u.mod2, vec3<f32>(0.0, 0.33, 0.67)) * o[2] * o[2] * 0.35;

    // level-set filigree: darken along iso-contours of the hue field, so
    // the network's topography reads as drawn lines
    let iso = abs(fract(o[0] * 6.0) - 0.5);
    col = col * (0.55 + 0.45 * smoothstep(0.04, 0.22, iso));

    let r2 = dot(in.uv, in.uv);
    col = col * (1.0 - 0.25 * r2);
    return vec4<f32>(col, 1.0);
}
