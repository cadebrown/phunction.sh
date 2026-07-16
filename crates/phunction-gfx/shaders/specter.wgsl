// specter — the live camera pulled through the phase pipeline. Kaleido
// folds, chromatic rotation, and phase-paint: you, but as light the
// machine dreamt. Requires WebGPU (external image copy).
//
//   mod0 = fold count        mod4 = bass  (radial pump)
//   mod1 = phase-paint       mod7 = air   (chromatic split)
//   mod2 = palette phase
//   mod3 = zoom

fn rot2s(p: vec2<f32>, a: f32) -> vec2<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec2<f32>(c * p.x - s * p.y, s * p.x + c * p.y);
}

fn kaleido(p0: vec2<f32>, n: f32, t: f32) -> vec2<f32> {
    var p = p0;
    let sector = TAU / n;
    var a = atan2(p.y, p.x) + t * 0.05;
    let r = length(p);
    a = abs(fract(a / sector - 0.5) - 0.5) * sector;
    return vec2<f32>(cos(a), sin(a)) * r;
}

fn sample_cam(p: vec2<f32>) -> vec3<f32> {
    // fold the plane back into texture space, mirrored at the edges
    let uv = clamp(p * 0.5 + 0.5, vec2<f32>(0.0), vec2<f32>(1.0));
    return textureSample(field_tex, field_samp, vec2<f32>(1.0 - uv.x, 1.0 - uv.y)).rgb;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let folds = floor(mix(3.0, 12.0, clamp(u.mod0, 0.0, 1.0)));
    let zoom = mix(1.6, 0.55, clamp(u.mod3, 0.0, 1.0));
    var p = in.uv * vec2<f32>(u.aspect, 1.0) * zoom;

    // the bass pumps the radius
    p = p * (1.0 - u.mod4 * 0.35 * sin(length(p) * 9.0 - u.time * 3.0));
    p = kaleido(p, folds, u.time);

    // chromatic split: r/g/b sampled at slightly rotated folds
    let split = 0.015 + u.mod7 * 0.12;
    let cr = sample_cam(rot2s(p, split)).r;
    let cg = sample_cam(p).g;
    let cb = sample_cam(rot2s(p, -split)).b;
    var cam = vec3<f32>(cr, cg, cb);

    // phase-paint: luminance drives the wheel, mod1 sets how deep in we go
    let luma = dot(cam, vec3<f32>(0.299, 0.587, 0.114));
    let painted = palette(luma * 0.85 + u.mod2 + u.time * 0.01, vec3<f32>(0.0, 0.33, 0.67)) * (0.2 + 1.1 * luma);
    var col = mix(cam, painted, clamp(u.mod1, 0.0, 1.0));

    let r2 = dot(in.uv, in.uv);
    col = col * (1.0 - 0.3 * r2);
    return vec4<f32>(col, 1.0);
}
