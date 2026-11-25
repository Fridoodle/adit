//! Built-in effects (SPEC E-BUILTIN), written against the same contract as user effects (E-1),
//! plus the renderer's own internal passes.

use adit_core::{EffectDef, EffectKind, ParamDef};

pub(crate) type Spec = (
    &'static str,
    EffectKind,
    &'static [(&'static str, f64)],
    &'static str,
);

use EffectKind::{Filter, Transition};

const BUILTINS: &[Spec] = &[
    (
        "exposure",
        Filter,
        &[("stops", 0.0)],
        "fn effect(uv: vec2f) -> vec4f {
    let c = src(uv);
    return vec4f(clamp(c.rgb * exp2(p.stops), vec3f(0.0), vec3f(1.0)), c.a);
}",
    ),
    (
        "brightness_contrast",
        Filter,
        &[("brightness", 0.0), ("contrast", 1.0)],
        "fn effect(uv: vec2f) -> vec4f {
    let c = src(uv);
    return vec4f(clamp((c.rgb - 0.5) * p.contrast + 0.5 + p.brightness, vec3f(0.0), vec3f(1.0)), c.a);
}",
    ),
    (
        "saturation",
        Filter,
        &[("amount", 1.0)],
        "fn effect(uv: vec2f) -> vec4f {
    let c = src(uv);
    let l = dot(c.rgb, vec3f(0.2126, 0.7152, 0.0722));
    return vec4f(clamp(mix(vec3f(l), c.rgb, p.amount), vec3f(0.0), vec3f(1.0)), c.a);
}",
    ),
    (
        "temperature",
        Filter,
        &[("temp", 0.0), ("tint", 0.0)],
        "fn effect(uv: vec2f) -> vec4f {
    let c = src(uv);
    let d = vec3f(0.1 * p.temp, 0.1 * p.tint, -0.1 * p.temp);
    return vec4f(clamp(c.rgb + d, vec3f(0.0), vec3f(1.0)), c.a);
}",
    ),
    (
        "levels",
        Filter,
        &[
            ("in_black", 0.0),
            ("in_white", 1.0),
            ("gamma", 1.0),
            ("out_black", 0.0),
            ("out_white", 1.0),
        ],
        "fn effect(uv: vec2f) -> vec4f {
    let c = src(uv);
    let n = clamp((c.rgb - p.in_black) / (p.in_white - p.in_black), vec3f(0.0), vec3f(1.0));
    let v = pow(n, vec3f(1.0 / p.gamma)) * (p.out_white - p.out_black) + p.out_black;
    return vec4f(clamp(v, vec3f(0.0), vec3f(1.0)), c.a);
}",
    ),
    (
        "lift_gamma_gain",
        Filter,
        &[
            ("lift_r", 0.0),
            ("lift_g", 0.0),
            ("lift_b", 0.0),
            ("gamma_r", 1.0),
            ("gamma_g", 1.0),
            ("gamma_b", 1.0),
            ("gain_r", 1.0),
            ("gain_g", 1.0),
            ("gain_b", 1.0),
        ],
        "fn effect(uv: vec2f) -> vec4f {
    let c = src(uv);
    let lift = vec3f(p.lift_r, p.lift_g, p.lift_b);
    let gamma = vec3f(p.gamma_r, p.gamma_g, p.gamma_b);
    let gain = vec3f(p.gain_r, p.gain_g, p.gain_b);
    let v = pow(clamp(c.rgb * gain + lift, vec3f(0.0), vec3f(1.0)), 1.0 / gamma);
    return vec4f(clamp(v, vec3f(0.0), vec3f(1.0)), c.a);
}",
    ),
    (
        "invert",
        Filter,
        &[("amount", 1.0)],
        "fn effect(uv: vec2f) -> vec4f {
    let c = src(uv);
    return vec4f(clamp(mix(c.rgb, 1.0 - c.rgb, p.amount), vec3f(0.0), vec3f(1.0)), c.a);
}",
    ),
    (
        "tint",
        Filter,
        &[("r", 1.0), ("g", 1.0), ("b", 1.0), ("amount", 1.0)],
        "fn effect(uv: vec2f) -> vec4f {
    let c = src(uv);
    let v = mix(c.rgb, c.rgb * vec3f(p.r, p.g, p.b), p.amount);
    return vec4f(clamp(v, vec3f(0.0), vec3f(1.0)), c.a);
}",
    ),
    (
        "hue_rotate",
        Filter,
        &[("degrees", 0.0)],
        "fn effect(uv: vec2f) -> vec4f {
    let c = src(uv);
    // Rodrigues rotation about the grey axis.
    let k = vec3f(0.57735027);
    let a = radians(p.degrees);
    let v = c.rgb * cos(a) + cross(k, c.rgb) * sin(a) + k * dot(k, c.rgb) * (1.0 - cos(a));
    return vec4f(clamp(v, vec3f(0.0), vec3f(1.0)), c.a);
}",
    ),
    (
        "flip",
        Filter,
        &[("horizontal", 0.0), ("vertical", 0.0)],
        "fn effect(uv: vec2f) -> vec4f {
    var q = uv;
    if (p.horizontal > 0.5) { q.x = 1.0 - q.x; }
    if (p.vertical > 0.5) { q.y = 1.0 - q.y; }
    return src(q);
}",
    ),
    (
        "rect",
        Filter,
        &[
            ("x", 0.25),
            ("y", 0.25),
            ("w", 0.5),
            ("h", 0.5),
            ("r", 1.0),
            ("g", 1.0),
            ("b", 1.0),
            ("a", 1.0),
        ],
        "fn over(s: vec4f, d: vec4f) -> vec4f {
    let a = s.a + d.a * (1.0 - s.a);
    if (a <= 0.0) { return vec4f(0.0); }
    return vec4f((s.rgb * s.a + d.rgb * d.a * (1.0 - s.a)) / a, a);
}
fn effect(uv: vec2f) -> vec4f {
    let c = src(uv);
    let inside = uv.x >= p.x && uv.x < p.x + p.w && uv.y >= p.y && uv.y < p.y + p.h;
    if (!inside) { return c; }
    return over(clamp(vec4f(p.r, p.g, p.b, p.a), vec4f(0.0), vec4f(1.0)), c);
}",
    ),
    (
        "ellipse",
        Filter,
        &[
            ("cx", 0.5),
            ("cy", 0.5),
            ("rx", 0.25),
            ("ry", 0.25),
            ("r", 1.0),
            ("g", 1.0),
            ("b", 1.0),
            ("a", 1.0),
        ],
        "fn over(s: vec4f, d: vec4f) -> vec4f {
    let a = s.a + d.a * (1.0 - s.a);
    if (a <= 0.0) { return vec4f(0.0); }
    return vec4f((s.rgb * s.a + d.rgb * d.a * (1.0 - s.a)) / a, a);
}
fn effect(uv: vec2f) -> vec4f {
    let c = src(uv);
    let d = (uv - vec2f(p.cx, p.cy)) / vec2f(p.rx, p.ry);
    if (dot(d, d) > 1.0) { return c; }
    return over(clamp(vec4f(p.r, p.g, p.b, p.a), vec4f(0.0), vec4f(1.0)), c);
}",
    ),
    (
        "linear_gradient",
        Filter,
        &[
            ("r0", 0.0),
            ("g0", 0.0),
            ("b0", 0.0),
            ("a0", 1.0),
            ("r1", 1.0),
            ("g1", 1.0),
            ("b1", 1.0),
            ("a1", 1.0),
            ("angle", 0.0),
        ],
        "fn effect(uv: vec2f) -> vec4f {
    // angle in degrees, clockwise (y down); 0 = left to right across the whole image.
    let a = radians(p.angle);
    let dir = vec2f(cos(a), sin(a));
    let t = clamp(dot(uv - 0.5, dir) / (abs(dir.x) + abs(dir.y)) + 0.5, 0.0, 1.0);
    let c0 = clamp(vec4f(p.r0, p.g0, p.b0, p.a0), vec4f(0.0), vec4f(1.0));
    let c1 = clamp(vec4f(p.r1, p.g1, p.b1, p.a1), vec4f(0.0), vec4f(1.0));
    let m = mix(vec4f(c0.rgb * c0.a, c0.a), vec4f(c1.rgb * c1.a, c1.a), t);
    if (m.a <= 0.0) { return vec4f(0.0); }
    return vec4f(m.rgb / m.a, m.a);
}",
    ),
    (
        "gaussian_blur",
        Filter,
        &[("radius", 4.0)],
        // shortcut: single-pass O(r^2) kernel capped at 24 px; split into two 1-D passes if big
        // radii are needed at video resolutions.
        "fn effect(uv: vec2f) -> vec4f {
    let r = min(p.radius, 24.0);
    if (r <= 0.0) { return src(uv); }
    let n = i32(ceil(r));
    let sigma = max(r / 2.0, 0.5);
    var acc = vec4f(0.0);
    var wsum = 0.0;
    for (var y = -n; y <= n; y++) {
        for (var x = -n; x <= n; x++) {
            let o = vec2f(f32(x), f32(y));
            let w = exp(-dot(o, o) / (2.0 * sigma * sigma));
            let c = src(uv + o / ctx.size);
            acc += vec4f(c.rgb * c.a, c.a) * w;
            wsum += w;
        }
    }
    let m = acc / wsum;
    if (m.a <= 0.0) { return vec4f(0.0); }
    return vec4f(m.rgb / m.a, m.a);
}",
    ),
    (
        "sharpen",
        Filter,
        &[("amount", 0.0)],
        "fn effect(uv: vec2f) -> vec4f {
    let c = src(uv);
    let t = 1.0 / ctx.size;
    let blur = (src(uv + vec2f(t.x, 0.0)) + src(uv - vec2f(t.x, 0.0))
        + src(uv + vec2f(0.0, t.y)) + src(uv - vec2f(0.0, t.y))) * 0.25;
    return vec4f(clamp(c.rgb + (c.rgb - blur.rgb) * p.amount, vec3f(0.0), vec3f(1.0)), c.a);
}",
    ),
    (
        "vignette",
        Filter,
        &[("amount", 0.5), ("radius", 0.75)],
        "fn effect(uv: vec2f) -> vec4f {
    let c = src(uv);
    // 0 at the centre, 1 at the corners; darkening starts at `radius`.
    let d = length(uv - 0.5) * 1.41421356;
    let v = 1.0 - p.amount * smoothstep(p.radius, p.radius + 0.5, d);
    return vec4f(clamp(c.rgb * v, vec3f(0.0), vec3f(1.0)), c.a);
}",
    ),
    (
        "pixelate",
        Filter,
        &[("size", 8.0)],
        "fn effect(uv: vec2f) -> vec4f {
    if (p.size <= 1.0) { return src(uv); }
    let px = uv * ctx.size;
    let cell = floor(px / p.size) * p.size;
    // Sample the texel centre nearest the block centre so blocks are flat.
    let centre = floor(min(cell + p.size * 0.5, ctx.size - 1.0)) + 0.5;
    return src(centre / ctx.size);
}",
    ),
    (
        "chroma_key",
        Filter,
        &[
            ("r", 0.0),
            ("g", 1.0),
            ("b", 0.0),
            ("threshold", 0.3),
            ("softness", 0.1),
        ],
        "fn effect(uv: vec2f) -> vec4f {
    let c = src(uv);
    let d = distance(c.rgb, vec3f(p.r, p.g, p.b));
    let keep = clamp((d - p.threshold) / max(p.softness, 0.00001), 0.0, 1.0);
    return vec4f(c.rgb, c.a * keep);
}",
    ),
    (
        "drop_shadow",
        Filter,
        &[("dx", 4.0), ("dy", 4.0), ("blur", 4.0), ("opacity", 0.5)],
        // shortcut: blur kernel capped at 16 px, same O(r^2) note as gaussian_blur.
        "fn effect(uv: vec2f) -> vec4f {
    let c = src(uv);
    let base = uv - vec2f(p.dx, p.dy) / ctx.size;
    let r = clamp(p.blur, 0.0, 16.0);
    let n = i32(ceil(r));
    let sigma = max(r / 2.0, 0.5);
    var acc = 0.0;
    var wsum = 0.0;
    for (var y = -n; y <= n; y++) {
        for (var x = -n; x <= n; x++) {
            let o = vec2f(f32(x), f32(y));
            let w = exp(-dot(o, o) / (2.0 * sigma * sigma));
            acc += src(base + o / ctx.size).a * w;
            wsum += w;
        }
    }
    let sa = clamp(acc / wsum * p.opacity, 0.0, 1.0);
    let a = c.a + sa * (1.0 - c.a);
    if (a <= 0.0) { return vec4f(0.0); }
    return vec4f(c.rgb * c.a / a, a);
}",
    ),
    (
        "stroke",
        Filter,
        &[("width", 2.0), ("r", 0.0), ("g", 0.0), ("b", 0.0), ("a", 1.0)],
        // shortcut: outline width capped at 16 px.
        "fn effect(uv: vec2f) -> vec4f {
    let c = src(uv);
    let wd = clamp(p.width, 0.0, 16.0);
    if (wd <= 0.0) { return c; }
    let n = i32(ceil(wd));
    var cover = 0.0;
    for (var y = -n; y <= n; y++) {
        for (var x = -n; x <= n; x++) {
            let o = vec2f(f32(x), f32(y));
            if (length(o) <= wd) { cover = max(cover, src(uv + o / ctx.size).a); }
        }
    }
    let s = clamp(vec4f(p.r, p.g, p.b, p.a * cover), vec4f(0.0), vec4f(1.0));
    let a = c.a + s.a * (1.0 - c.a);
    if (a <= 0.0) { return vec4f(0.0); }
    return vec4f((c.rgb * c.a + s.rgb * s.a * (1.0 - c.a)) / a, a);
}",
    ),
    (
        "noise",
        Filter,
        &[("amount", 0.1), ("seed", 0.0)],
        "fn pcg(v: u32) -> u32 {
    let s = v * 747796405u + 2891336453u;
    let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    return (w >> 22u) ^ w;
}
fn effect(uv: vec2f) -> vec4f {
    let c = src(uv);
    let px = vec2u(uv * ctx.size);
    let h = pcg(px.x + pcg(px.y + pcg(bitcast<u32>(p.seed) + pcg(bitcast<u32>(ctx.frame)))));
    let n = f32(h) / 4294967295.0 * 2.0 - 1.0;
    return vec4f(clamp(c.rgb + n * p.amount, vec3f(0.0), vec3f(1.0)), c.a);
}",
    ),
    (
        "cross_dissolve",
        Transition,
        &[],
        "fn transition(uv: vec2f) -> vec4f {
    let a = src_a(uv);
    let b = src_b(uv);
    let m = mix(vec4f(a.rgb * a.a, a.a), vec4f(b.rgb * b.a, b.a), ctx.progress);
    if (m.a <= 0.0) { return vec4f(0.0); }
    return vec4f(m.rgb / m.a, m.a);
}",
    ),
    (
        "dip_to_black",
        Transition,
        &[],
        "fn transition(uv: vec2f) -> vec4f {
    let a = src_a(uv);
    let b = src_b(uv);
    let black = vec4f(0.0, 0.0, 0.0, 1.0);
    let t = ctx.progress;
    var m: vec4f;
    if (t < 0.5) {
        m = mix(vec4f(a.rgb * a.a, a.a), black, 2.0 * t);
    } else {
        m = mix(black, vec4f(b.rgb * b.a, b.a), 2.0 * t - 1.0);
    }
    if (m.a <= 0.0) { return vec4f(0.0); }
    return vec4f(m.rgb / m.a, m.a);
}",
    ),
    (
        "wipe",
        Transition,
        &[],
        "fn transition(uv: vec2f) -> vec4f {
    if (uv.x < ctx.progress) { return src_b(uv); }
    return src_a(uv);
}",
    ),
    (
        "push",
        Transition,
        &[],
        "fn transition(uv: vec2f) -> vec4f {
    let t = ctx.progress;
    if (uv.x < 1.0 - t) { return src_a(vec2f(uv.x + t, uv.y)); }
    return src_b(vec2f(uv.x - (1.0 - t), uv.y));
}",
    ),
];

fn to_def(&(name, kind, params, wgsl): &Spec) -> EffectDef {
    EffectDef {
        name: name.into(),
        kind,
        wgsl: wgsl.into(),
        params: params
            .iter()
            .map(|&(name, default)| ParamDef {
                name: name.into(),
                default,
                // min/max are UI hints only (E-6); a generous range around the default.
                min: default.min(0.0) - 100.0,
                max: default.max(1.0) + 100.0,
            })
            .collect(),
    }
}

pub(crate) fn builtin_effects() -> Vec<EffectDef> {
    BUILTINS.iter().map(to_def).collect()
}

/// Renderer passes. Transform: canvas-sized output from the clip image (V-5, V-6, V-7 opacity).
pub(crate) const TRANSFORM: Spec = (
    "adit_transform",
    Filter,
    &[
        ("m00", 0.0),
        ("m01", 0.0),
        ("m10", 0.0),
        ("m11", 0.0),
        ("ax", 0.0),
        ("ay", 0.0),
        ("cx", 0.0),
        ("cy", 0.0),
        ("crop_l", 0.0),
        ("crop_t", 0.0),
        ("crop_r", 0.0),
        ("crop_b", 0.0),
        ("opacity", 0.0),
    ],
    "fn effect(uv: vec2f) -> vec4f {
    let d = uv * ctx.size - vec2f(p.cx, p.cy);
    let q = vec2f(p.ax, p.ay) + vec2f(p.m00 * d.x + p.m01 * d.y, p.m10 * d.x + p.m11 * d.y);
    if (q.x < p.crop_l || q.x >= p.crop_r || q.y < p.crop_t || q.y >= p.crop_b) { return vec4f(0.0); }
    let c = src(q / vec2f(textureDimensions(adit_tex_a)));
    return vec4f(c.rgb, c.a * p.opacity);
}",
);

/// V-7: canvas (a) and layer (b) to the new canvas.
pub(crate) const BLEND: Spec = (
    "adit_blend",
    Transition,
    &[("mode", 0.0)],
    "fn blend(d: vec3f, s: vec3f) -> vec3f {
    switch (i32(p.mode)) {
        case 1: { return min(vec3f(1.0), d + s); }
        case 2: { return d * s; }
        case 3: { return 1.0 - (1.0 - d) * (1.0 - s); }
        case 4: { return select(1.0 - 2.0 * (1.0 - d) * (1.0 - s), 2.0 * d * s, d < vec3f(0.5)); }
        case 5: { return min(d, s); }
        case 6: { return max(d, s); }
        case 7: { return abs(d - s); }
        default: { return s; }
    }
}
fn transition(uv: vec2f) -> vec4f {
    let d = src_a(uv);
    let s = src_b(uv);
    let s2 = (1.0 - d.a) * s.rgb + d.a * blend(d.rgb, s.rgb);
    let a = s.a + d.a * (1.0 - s.a);
    if (a <= 0.0) { return vec4f(0.0); }
    return vec4f((s.a * s2 + (1.0 - s.a) * d.a * d.rgb) / a, a);
}",
);

/// V-8: `mix(canvas, result, amount)`.
pub(crate) const MIX: Spec = (
    "adit_mix",
    Transition,
    &[("amount", 0.0)],
    "fn transition(uv: vec2f) -> vec4f { return mix(src_a(uv), src_b(uv), p.amount); }",
);

/// X-1 output scaling, bilinear, edges clamped so the border does not fade.
pub(crate) const SCALE: Spec = (
    "adit_scale",
    Filter,
    &[],
    "fn effect(uv: vec2f) -> vec4f {
    let h = 0.5 / vec2f(textureDimensions(adit_tex_a));
    return src(clamp(uv, h, 1.0 - h));
}",
);

pub(crate) fn internal(spec: &Spec) -> EffectDef {
    to_def(spec)
}
