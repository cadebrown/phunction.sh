// lenia (present pass) — the living field as bioluminescent neural flow.
// Unlike petri's matte coral, here the membrane value drives a hue journey
// (deep indigo troughs → violet filaments → teal-white synapse nodes), a
// gradient reads as glassy relief, and a cheap bloom lifts the brightest
// junctions off the black so the whole field glows and breathes.

fn field(uv: vec2<f32>) -> f32 {
    return textureSample(state_tex, state_samp, uv).r;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let uv = in.uv * 0.5 + 0.5;
    let a = field(uv);

    // glassy relief from the state gradient (both axes read as side light)
    let ex = vec2<f32>(1.0 / 512.0, 0.0);
    let ey = vec2<f32>(0.0, 1.0 / 288.0);
    let gx = field(uv + ex) - field(uv - ex);
    let gy = field(uv + ey) - field(uv - ey);
    let light = clamp(0.5 + (gx * 0.7 - gy * 0.5) * 5.0, 0.0, 1.5);

    // cheap bloom: a wider blur of the field lifts the bright junctions
    var bloom = 0.0;
    bloom = bloom + field(uv + ex * 3.0) + field(uv - ex * 3.0);
    bloom = bloom + field(uv + ey * 3.0) + field(uv - ey * 3.0);
    bloom = bloom + field(uv + (ex + ey) * 2.5) + field(uv - (ex + ey) * 2.5);
    bloom = bloom / 6.0;

    let membrane = smoothstep(0.05, 0.65, a);
    // the hue journey: value walks the canon wheel, so troughs and crests
    // are different colors — depth petri's single-phase mapping never has
    let tone = palette(a * 0.42 + u.mod2 + u.time * 0.004, vec3<f32>(0.0, 0.33, 0.67));
    let hot = palette(u.mod2 + 0.15, vec3<f32>(0.0, 0.33, 0.67)); // synapse tint

    var col = tone * (0.05 + membrane * (0.3 + 0.7 * light));
    // synapse nodes: where the field peaks, a bright warm-teal core
    col = col + hot * pow(membrane, 4.0) * 0.5;
    // bloom halo
    col = col + tone * bloom * bloom * 0.35;
    col = max(col, vec3<f32>(0.012, 0.008, 0.035));
    let r2 = dot(in.uv, in.uv);
    col = col * (1.0 - 0.24 * r2);
    return vec4<f32>(col, 1.0);
}
