//! Effect shader contract (SPEC E-1, E-2, E-4): prelude assembly and naga validation.

use adit_core::{EffectDef, EffectKind};
use wgpu::naga;

/// Prelude globals adit declares itself; user code may declare no other bound globals.
const OWN_BINDINGS: [&str; 4] = ["ctx", "p", "adit_tex_a", "adit_tex_b"];
const MAX_PARAMS: usize = 64;

const PRELUDE: &str = r#"
struct AditCtx { size: vec2f, time: f32, duration: f32, progress: f32, frame: f32 }
@group(0) @binding(0) var<uniform> ctx: AditCtx;
@group(0) @binding(1) var<uniform> p: AditParams;
@group(0) @binding(2) var adit_tex_a: texture_2d<f32>;
@group(0) @binding(3) var adit_tex_b: texture_2d<f32>;

fn adit_texel(t: texture_2d<f32>, c: vec2i) -> vec4f {
    let d = vec2i(textureDimensions(t));
    if (any(c < vec2i(0)) || any(c >= d)) { return vec4f(0.0); }
    let v = textureLoad(t, c, 0);
    return vec4f(v.rgb * v.a, v.a);
}

// Bilinear on premultiplied texels so transparent neighbours do not darken edges.
fn adit_sample(t: texture_2d<f32>, uv: vec2f) -> vec4f {
    let x = uv * vec2f(textureDimensions(t)) - 0.5;
    let f = floor(x);
    let w = x - f;
    let i = vec2i(f);
    let top = mix(adit_texel(t, i), adit_texel(t, i + vec2i(1, 0)), w.x);
    let bot = mix(adit_texel(t, i + vec2i(0, 1)), adit_texel(t, i + vec2i(1, 1)), w.x);
    let m = mix(top, bot, w.y);
    if (m.a <= 0.0) { return vec4f(0.0); }
    return vec4f(m.rgb / m.a, m.a);
}

"#;

const VERTEX: &str = r#"
@vertex fn adit_vs(@builtin(vertex_index) i: u32) -> @builtin(position) vec4f {
    let xy = vec2f(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4f(xy.x * 2.0 - 1.0, 1.0 - xy.y * 2.0, 0.0, 1.0);
}
"#;

const FILTER_PRELUDE: &str = "
fn src(uv: vec2f) -> vec4f { return adit_sample(adit_tex_a, uv); }
";

const TRANSITION_PRELUDE: &str = "
fn src_a(uv: vec2f) -> vec4f { return adit_sample(adit_tex_a, uv); }
fn src_b(uv: vec2f) -> vec4f { return adit_sample(adit_tex_b, uv); }
";

fn entry(kind: EffectKind) -> &'static str {
    match kind {
        EffectKind::Filter => "effect",
        EffectKind::Transition => "transition",
    }
}

/// User code first (so naga line numbers match the author's), then the prelude; without the
/// fragment entry point when `main` is false.
fn assemble_parts<'a>(
    kind: EffectKind,
    user: &str,
    params: impl Iterator<Item = &'a str>,
    main: bool,
) -> String {
    let mut fields: Vec<String> = params.map(|n| format!("{n}: f32")).collect();
    if fields.is_empty() {
        fields.push("adit_unused: f32".into());
    }
    let mut s = format!(
        "{user}\n// ---- adit prelude\nstruct AditParams {{ {} }}\n{PRELUDE}{}",
        fields.join(", "),
        match kind {
            EffectKind::Filter => FILTER_PRELUDE,
            EffectKind::Transition => TRANSITION_PRELUDE,
        }
    );
    if main {
        s += VERTEX;
        s += &format!(
            "@fragment fn adit_fs(@builtin(position) pos: vec4f) -> @location(0) vec4f {{ return {}(pos.xy / ctx.size); }}\n",
            entry(kind)
        );
    }
    s
}

/// The shader the renderer compiles for `def`.
pub(crate) fn assemble(def: &EffectDef) -> String {
    assemble_parts(
        def.kind,
        &def.wgsl,
        def.params.iter().map(|p| p.name.as_str()),
        true,
    )
}

const KEYWORDS: &[&str] = &[
    "alias",
    "break",
    "case",
    "const",
    "const_assert",
    "continue",
    "continuing",
    "default",
    "diagnostic",
    "discard",
    "else",
    "enable",
    "false",
    "fn",
    "for",
    "if",
    "let",
    "loop",
    "override",
    "requires",
    "return",
    "struct",
    "switch",
    "true",
    "var",
    "while",
];

fn check_param_names(def: &EffectDef) -> Result<(), String> {
    if def.params.len() > MAX_PARAMS {
        return Err(format!(
            "{} parameters, at most {MAX_PARAMS} allowed",
            def.params.len()
        ));
    }
    for (i, p) in def.params.iter().enumerate() {
        let n = p.name.as_str();
        let ident = n
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
            && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            && n != "_"
            && !n.starts_with("__");
        if !ident || KEYWORDS.contains(&n) {
            return Err(format!(
                "parameter name {n:?} is not a valid WGSL identifier"
            ));
        }
        if def.params[..i].iter().any(|q| q.name == n) {
            return Err(format!("parameter name {n:?} is declared twice"));
        }
    }
    Ok(())
}

fn is_type(module: &naga::Module, ty: naga::Handle<naga::Type>, size: naga::VectorSize) -> bool {
    matches!(
        module.types[ty].inner,
        naga::TypeInner::Vector { size: s, scalar } if s == size && scalar == naga::Scalar::F32
    )
}

fn validate_inner(def: &EffectDef) -> Result<(), String> {
    check_param_names(def)?;
    let name = entry(def.kind);
    let signature = format!("`fn {name}(uv: vec2f) -> vec4f`");
    // Parsed without adit's entry point so a missing or mistyped `effect` gets its own message.
    let source = assemble_parts(
        def.kind,
        &def.wgsl,
        def.params.iter().map(|p| p.name.as_str()),
        false,
    );
    let module = naga::front::wgsl::parse_str(&source).map_err(|e| {
        let at = e
            .location(&source)
            .map(|l| format!(" (line {}:{})", l.line_number, l.line_position))
            .unwrap_or_default();
        let labels: Vec<&str> = e
            .labels()
            .map(|(_, l)| l)
            .filter(|l| !l.is_empty())
            .collect();
        let mut m = format!("WGSL error{at}: {}", e.message());
        if !labels.is_empty() {
            m += &format!(" ({})", labels.join("; "));
        }
        m
    })?;
    // H-2: an override without a pipeline constant fails at pipeline creation, after validation.
    if let Some((_, o)) = module.overrides.iter().next() {
        return Err(format!(
            "effect code must not declare `override` constants (found `{}`); use params instead",
            o.name.as_deref().unwrap_or("?")
        ));
    }
    if let Some(ep) = module.entry_points.first() {
        return Err(format!(
            "effect code must not declare entry points (found `{}`); adit supplies them",
            ep.name
        ));
    }
    for (_, g) in module.global_variables.iter() {
        let n = g.name.as_deref().unwrap_or("");
        if g.binding.is_some() && !OWN_BINDINGS.contains(&n) {
            return Err(format!(
                "effect code must not declare bindings (found `{n}`); use params instead"
            ));
        }
    }
    let f = module
        .functions
        .iter()
        .map(|(_, f)| f)
        .find(|f| f.name.as_deref() == Some(name))
        .ok_or_else(|| format!("missing {signature}"))?;
    let ok = f.arguments.len() == 1
        && is_type(&module, f.arguments[0].ty, naga::VectorSize::Bi)
        && f.result
            .as_ref()
            .is_some_and(|r| is_type(&module, r.ty, naga::VectorSize::Quad));
    if !ok {
        return Err(format!("`{name}` must have the signature {signature}"));
    }
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::default(),
    )
    .validate(&module)
    .map_err(|e| {
        let mut m = e.emit_to_string(&source);
        // A huge source line can end up in the snippet; keep the message readable.
        if m.len() > 4000 {
            let mut cut = 4000;
            while !m.is_char_boundary(cut) {
                cut -= 1;
            }
            m.truncate(cut);
        }
        format!("WGSL validation error: {m}")
    })?;
    Ok(())
}

const MAX_WGSL_BYTES: usize = 65_536;
const MAX_NESTING: usize = 64;
/// naga (and wgpu's backend compile) recurse on the caller's stack, also over operator chains the
/// bracket limit does not see: 65,536 bytes of `!!!...x` or `1+1+...` needed 256-512 MiB in a
/// debug build (measured). This is a reservation; only touched pages are committed.
const COMPILER_STACK: usize = 1 << 30;

/// Runs `f` on a thread with `COMPILER_STACK` and waits for it; a panic becomes `Err`.
pub(crate) fn on_big_stack<T: Send>(f: impl FnOnce() -> T + Send) -> Result<T, String> {
    std::thread::scope(|s| {
        std::thread::Builder::new()
            .stack_size(COMPILER_STACK)
            .spawn_scoped(s, f)
            .map_err(|e| format!("cannot start the shader compiler thread: {e}"))?
            .join()
            .map_err(|_| "shader compiler crashed on this input".to_string())
    })
}

/// Same set as naga's lexer, so a comment ends exactly where naga resumes parsing.
fn is_comment_end(c: char) -> bool {
    matches!(c, '\u{a}'..='\u{d}' | '\u{85}' | '\u{2028}' | '\u{2029}')
}

/// Deepest `(` / `[` / `{` nesting (H-1). Comments are skipped the way naga's lexer skips them
/// (line comments end at any line break, block comments nest), so a bracket in a comment neither
/// counts nor can hide code from the check. WGSL has no string literals.
fn nesting_depth(s: &str) -> usize {
    let (mut depth, mut max) = (0usize, 0usize);
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '(' | '[' | '{' => {
                depth += 1;
                max = max.max(depth);
            }
            ')' | ']' | '}' => depth = depth.saturating_sub(1),
            '/' => match chars.clone().next() {
                Some('/') => {
                    chars.find(|&c| is_comment_end(c));
                }
                Some('*') => {
                    chars.next();
                    // Mirrors naga: the character right after `/*` only seeds `prev`.
                    let mut prev = chars.next();
                    let mut open = 1;
                    for c in chars.by_ref() {
                        match (prev, c) {
                            (Some('*'), '/') => {
                                prev = None;
                                open -= 1;
                                if open == 0 {
                                    break;
                                }
                            }
                            (Some('/'), '*') => {
                                prev = None;
                                open += 1;
                            }
                            _ => prev = Some(c),
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    max
}

pub(crate) fn validate(def: &EffectDef) -> Result<(), String> {
    // H-1: checked before naga sees the source, so its recursion stays bounded.
    if def.wgsl.len() > MAX_WGSL_BYTES {
        return Err(format!(
            "effect code is {} bytes, at most {MAX_WGSL_BYTES} allowed",
            def.wgsl.len()
        ));
    }
    let depth = nesting_depth(&def.wgsl);
    if depth > MAX_NESTING {
        return Err(format!(
            "effect code nests brackets {depth} deep, at most {MAX_NESTING} allowed"
        ));
    }
    on_big_stack(|| validate_inner(def))?
}

#[cfg(test)]
mod tests {
    use super::nesting_depth;

    #[test]
    fn nesting_depth_skips_comments_like_naga() {
        assert_eq!(nesting_depth("fn f() { a[(1)]; }"), 3);
        assert_eq!(nesting_depth("// ((((\n("), 1);
        assert_eq!(nesting_depth("// ((((\u{2028}(("), 2);
        assert_eq!(nesting_depth("/* ( /* ( */ ( */ ("), 1);
        // `/**/` closes; the `*` after `/*` cannot also open a nested comment.
        assert_eq!(nesting_depth("/**/ (("), 2);
        assert_eq!(nesting_depth("/*/ ( */ ("), 1);
        assert_eq!(nesting_depth("/* unterminated ((("), 0);
        assert_eq!(nesting_depth("a / (b)"), 1);
    }
}
