//! T-4 time strings, fps strings, and the JSON pre-pass over edit commands.

use adit_core::{FLICKS_PER_SECOND, Rational, Time};
use serde_json::Value;

/// Command fields that hold a time (T-4).
const TIME_FIELDS: [&str; 7] = [
    "start",
    "end",
    "duration",
    "time",
    "to",
    "delta",
    "source_in",
];

/// SPEC Q4 `frame_to_time`, overflow-checked because the input is untrusted.
fn frames_to_time(n: i128, fps: Rational) -> Option<Time> {
    if fps.num == 0 {
        return None;
    }
    let t = n
        .checked_mul(i128::from(FLICKS_PER_SECOND))?
        .checked_mul(i128::from(fps.den))?
        .div_euclid(i128::from(fps.num));
    Time::try_from(t).ok()
}

/// `-?digits[.digits]`, at least one digit, nothing else.
fn is_decimal(s: &str) -> bool {
    let d = s.strip_prefix('-').unwrap_or(s);
    d.chars().any(|c| c.is_ascii_digit())
        && d.chars().all(|c| c.is_ascii_digit() || c == '.')
        && d.matches('.').count() <= 1
}

/// `-?digits`, at most 15 digits.
fn parse_int(s: &str) -> Option<i128> {
    let d = s.strip_prefix('-').unwrap_or(s);
    if d.is_empty() || d.len() > 15 || !d.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}

/// A time value: integer flicks, `"1.5s"`, `"45f"` or `"00:00:01:15"`. `fps` is the frame rate for
/// frame-based strings (`None` = no sequence available).
pub fn parse_time(field: &str, v: &Value, fps: Option<Rational>) -> Result<Time, String> {
    let bad = |why: &str| format!("invalid time for `{field}` ({v}): {why}");
    let s = match v {
        Value::Number(n) => {
            return n
                .as_i64()
                .ok_or_else(|| bad("a number must be an integer count of flicks"));
        }
        Value::String(s) => s.as_str(),
        _ => {
            return Err(bad(
                "expected integer flicks or a string like \"1.5s\", \"45f\", \"00:00:01:15\"",
            ));
        }
    };
    let fps_or_err = || {
        fps.ok_or_else(|| {
            bad("frame-based times need a sequence for the frame rate; there is none")
        })
    };
    if let Some(n) = s.strip_suffix('s') {
        if !is_decimal(n) {
            return Err(bad("expected like \"1.5s\""));
        }
        let t =
            (n.parse::<f64>().map_err(|_| bad("not a number"))? * FLICKS_PER_SECOND as f64).round();
        if !t.is_finite() || t.abs() >= i64::MAX as f64 {
            return Err(bad("out of range"));
        }
        Ok(t as Time)
    } else if let Some(n) = s.strip_suffix('f') {
        let n = parse_int(n).ok_or_else(|| bad("expected like \"45f\""))?;
        frames_to_time(n, fps_or_err()?).ok_or_else(|| bad("out of range"))
    } else if s.contains(':') {
        let p: Vec<Option<i128>> = s
            .split(':')
            .map(|x| parse_int(x).filter(|v| *v >= 0 && x.len() <= 9))
            .collect();
        let [Some(h), Some(m), Some(sec), Some(f)] = p[..] else {
            return Err(bad("expected 4 non-negative parts like \"00:00:01:15\""));
        };
        let fps = fps_or_err()?;
        // Non-drop: ceil(fps) frames per timecode second.
        let per_s = (i128::from(fps.num) + i128::from(fps.den) - 1) / i128::from(fps.den.max(1));
        let frames = ((h * 60 + m) * 60 + sec) * per_s + f;
        frames_to_time(frames, fps).ok_or_else(|| bad("out of range"))
    } else {
        Err(bad(
            "expected integer flicks or a string like \"1.5s\", \"45f\", \"00:00:01:15\"",
        ))
    }
}

/// Rewrites the time fields of one command object (and of nested `batch` commands) in place.
pub fn convert_times(cmd: &mut Value, fps: Option<Rational>) -> Result<(), String> {
    let Some(o) = cmd.as_object_mut() else {
        return Ok(()); // serde reports a non-object command
    };
    for k in TIME_FIELDS {
        if let Some(v) = o.get_mut(k).filter(|v| !v.is_null()) {
            *v = Value::from(parse_time(k, v, fps)?);
        }
    }
    // add_media: spec.duration
    if let Some(v) = o
        .get_mut("spec")
        .and_then(|s| s.get_mut("duration"))
        .filter(|v| v.is_string())
    {
        *v = Value::from(parse_time("duration", v, fps)?);
    }
    if let Some(Value::Array(subs)) = o.get_mut("commands") {
        for s in subs {
            convert_times(s, fps)?;
        }
    }
    Ok(())
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// `"30"`, `"29.97"` (NTSC rates become n*1000/1001), `"12.5"` or `"num/den"`.
pub fn parse_fps(s: &str) -> Result<Rational, String> {
    let bad = || format!("invalid fps {s:?}: use \"30\", \"29.97\" or \"num/den\"");
    let (num, den) = if let Some((a, b)) = s.split_once('/') {
        (
            a.parse::<u32>().map_err(|_| bad())?,
            b.parse::<u32>().map_err(|_| bad())?,
        )
    } else {
        if !is_decimal(s) || s.starts_with('-') {
            return Err(bad());
        }
        let x: f64 = s.parse().map_err(|_| bad())?;
        let r = x * 1.001;
        if x.fract() != 0.0 && x < 1000.0 && (r - r.round()).abs() < 0.001 {
            (r.round() as u32 * 1000, 1001)
        } else {
            let (a, b) = s.split_once('.').unwrap_or((s, ""));
            if b.len() > 6 {
                return Err(bad());
            }
            let num: u64 = format!("{a}{b}").parse().map_err(|_| bad())?;
            let den = 10u64.pow(b.len() as u32);
            let g = gcd(num, den).max(1);
            (
                u32::try_from(num / g).map_err(|_| bad())?,
                u32::try_from(den / g).map_err(|_| bad())?,
            )
        }
    };
    if num == 0 || den == 0 {
        return Err(bad());
    }
    Ok(Rational { num, den })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn fps_strings() {
        let r = |s| parse_fps(s).map(|r| (r.num, r.den));
        assert_eq!(r("30"), Ok((30, 1)));
        assert_eq!(r("29.97"), Ok((30000, 1001)));
        assert_eq!(r("23.976"), Ok((24000, 1001)));
        assert_eq!(r("59.94"), Ok((60000, 1001)));
        assert_eq!(r("12.5"), Ok((25, 2)));
        assert_eq!(r("24000/1001"), Ok((24000, 1001)));
        for bad in ["", "abc", "0", "-30", "30/0", "1.2.3"] {
            assert!(r(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn time_strings() {
        let fps = Some(Rational { num: 30, den: 1 });
        let t = |v: Value| parse_time("x", &v, fps);
        assert_eq!(t(json!("1.5s")), Ok(1_058_400_000));
        assert_eq!(t(json!("-3f")), Ok(-70_560_000));
        assert_eq!(t(json!("00:00:01:15")), Ok(1_058_400_000));
        assert_eq!(t(json!(7)), Ok(7));
        for bad in ["", "s", "1..5s", "--1s", "1s2", "0x10s", "00:00:01", "1.5"] {
            assert!(t(json!(bad)).is_err(), "{bad}");
        }
        assert!(t(json!(1.5)).is_err());
        assert!(parse_time("x", &json!("3f"), None).is_err());
        assert!(t(json!("99999999999999f")).is_err(), "overflow is an error");
    }
}
