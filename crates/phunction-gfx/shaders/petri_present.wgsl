// petri (present pass) — map the dish's chemistry to the canon wheel:
// chemical B is the organism, its gradient the lighting, mod2 the hue.

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let uv = in.uv * 0.5 + 0.5;
    let s = textureSample(state_tex, state_samp, uv).rg;
    let organism = smoothstep(0.12, 0.42, s.y);
    // cheap relief: B's horizontal gradient reads as side light
    let e = vec2<f32>(1.0 / 512.0, 0.0);
    let gx = textureSample(state_tex, state_samp, uv + e).g
        - textureSample(state_tex, state_samp, uv - e).g;
    let tone = palette(organism * 0.55 + u.mod2 + u.time * 0.002, vec3<f32>(0.0));
    var col = tone * (0.08 + organism * (0.8 + gx * 3.0));
    col = max(col, vec3<f32>(0.02, 0.012, 0.045));
    let r2 = dot(in.uv, in.uv);
    col = col * (1.0 - 0.22 * r2);
    return vec4<f32>(col, 1.0);
}
