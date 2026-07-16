// hopf — the Hopf fibration: S³ is a sphere of circles, one through every
// point, no two touching. Base points spiral down S² (golden angle); each
// fiber stereographically projects to a circle in R³ (verified natively:
// equidistant + coplanar to 1e-6). The base sphere precesses, so the
// nested tori of fibers slowly braid. The view ray marches a bounded
// shell, accumulating additive glow from every fiber tube.
//
//   mod0 = fibers (latitude band)       mod4 = bass  (fibers brighten)
//   mod1 = precession depth             mod7 = air
//   mod2 = palette phase
//   mod3 = orbit speed

const N_FIBERS: i32 = 24;

// distance from p to the circle (center c, unit normal n, radius r)
fn circle_dist(p: vec3<f32>, c: vec3<f32>, n: vec3<f32>, r: f32) -> f32 {
    let d = p - c;
    let dn = dot(d, n);
    let dr = length(d - n * dn) - r;
    return sqrt(dn * dn + dr * dr);
}

fn fiber_point(theta: f32, phi: f32, xi: f32) -> vec3<f32> {
    let c = cos(theta * 0.5);
    let s = sin(theta * 0.5);
    let x1 = c * cos(xi + phi);
    let x2 = c * sin(xi + phi);
    let x3 = s * cos(xi);
    let x4 = s * sin(xi);
    return vec3<f32>(x1, x2, x3) / max(1.0 - x4, 1e-4);
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let t = u.time * mix(0.01, 0.08, clamp(u.mod3, 0.0, 1.0));

    // orbit camera, close enough that the fiber torus fills the frame
    let dist = 4.2;
    let eye = vec3<f32>(cos(t * 0.4) * dist, 1.4 + 1.0 * sin(t * 0.17), sin(t * 0.4) * dist);
    let fwd = normalize(-eye);
    let right = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), fwd));
    let up = cross(fwd, right);
    let uv = in.uv * vec2<f32>(u.aspect, 1.0);
    let rd = normalize(fwd * 1.4 + right * uv.x + up * uv.y);

    let band = mix(0.35, 0.95, clamp(u.mod0, 0.0, 1.0));
    let wob = mix(0.1, 0.55, clamp(u.mod1, 0.0, 1.0));

    // precompute each fiber's circumcircle (center, unit normal, radius, hue)
    var cc: array<vec3<f32>, N_FIBERS>;
    var cn: array<vec3<f32>, N_FIBERS>;
    var cr: array<f32, N_FIBERS>;
    for (var i = 0; i < N_FIBERS; i = i + 1) {
        let fi = f32(i) / f32(N_FIBERS);
        let theta = acos(1.0 - band * (0.3 + fi * 1.4)) + wob * 0.3 * sin(t * 0.7 + fi * 6.0);
        let phi = fi * 39.9264 + t * (0.5 + wob) + wob * sin(t * 0.31 + fi * 12.0);
        let p0 = fiber_point(theta, phi, 0.0);
        let p1 = fiber_point(theta, phi, 2.0944);
        let p2 = fiber_point(theta, phi, 4.1888);
        let e1 = p1 - p0;
        let e2 = p2 - p0;
        var n = cross(e1, e2);
        let n2 = max(dot(n, n), 1e-9);
        let c = p0 - (dot(e1, e1) * cross(e2, n) - dot(e2, e2) * cross(e1, n)) / (2.0 * n2);
        cc[i] = c;
        cn[i] = n / sqrt(n2);
        cr[i] = length(p0 - c);
    }

    // march the ray through the fibration's bounding shell, accumulating
    // additive glow from every fiber tube — this gives continuous luminous
    // circles instead of the sparse dots a single closest-sample gives
    var col = vec3<f32>(0.0);
    // enter/exit a sphere of radius 5 around the origin
    let b = dot(eye, rd);
    let disc = b * b - (dot(eye, eye) - 25.0);
    if (disc > 0.0) {
        let sq = sqrt(disc);
        var tt = max(-b - sq, 0.1);
        let t_far = -b + sq;
        let steps = 40;
        let dtm = (t_far - tt) / f32(steps);
        for (var s = 0; s < steps; s = s + 1) {
            let p = eye + rd * tt;
            for (var i = 0; i < N_FIBERS; i = i + 1) {
                // fibers near the projection pole blow up to near-lines;
                // fade them so a few giants don't wash the frame
                let big = smoothstep(9.0, 3.0, cr[i]);
                let d = circle_dist(p, cc[i], cn[i], cr[i]);
                let fi = f32(i) / f32(N_FIBERS);
                let tone = palette(fi * 0.6 + u.mod2, vec3<f32>(0.0, 0.33, 0.67));
                col = col + tone * (0.0026 / (d * d + 0.0009)) * dtm * big * (1.0 + u.mod4 * 1.3);
            }
            tt = tt + dtm;
        }
    }
    let air = palette(u.mod2 + 0.5, vec3<f32>(0.0, 0.33, 0.67));
    col = col + air * 0.03 * (1.0 + u.mod7);
    let r2 = dot(in.uv, in.uv);
    col = col * (1.0 - 0.28 * r2);
    return vec4<f32>(col, 1.0);
}
