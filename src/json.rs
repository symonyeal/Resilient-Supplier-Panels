//! JSON text as Python's `json.dumps` writes it (ensure_ascii, float repr), so documents keep
//! the bytes of the 0.2 Python release; parsing accepts UTF-8 (with or without BOM) and UTF-16.

pub use serde_json::{json, Map, Value};

#[derive(Clone, Copy)]
pub enum Style {
    /// indent = n, separators (",", ": ").
    Indent(usize),
    /// separators (",", ":"), one line.
    Compact,
}

/// Python `repr(float)`: shortest round-trip digits, fixed notation for −4 < decpt ≤ 16.
pub fn float(x: f64) -> String {
    if x.is_nan() {
        return "NaN".into();
    }
    if x.is_infinite() {
        return if x > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    if x == 0.0 {
        return if x.is_sign_negative() { "-0.0".into() } else { "0.0".into() };
    }
    let s = format!("{:e}", x.abs());
    let (m, e) = s.split_once('e').unwrap();
    let digits: String = m.chars().filter(|c| *c != '.').collect();
    let decpt = e.parse::<i32>().unwrap() + 1;
    let n = digits.len() as i32;
    let body = if decpt <= -4 || decpt > 16 {
        let mant = if n == 1 { digits.clone() } else { format!("{}.{}", &digits[..1], &digits[1..]) };
        let ex = decpt - 1;
        format!("{mant}e{}{:02}", if ex < 0 { '-' } else { '+' }, ex.abs())
    } else if decpt <= 0 {
        format!("0.{}{digits}", "0".repeat((-decpt) as usize))
    } else if decpt >= n {
        format!("{digits}{}.0", "0".repeat((decpt - n) as usize))
    } else {
        format!("{}.{}", &digits[..decpt as usize], &digits[decpt as usize..])
    };
    if x < 0.0 { format!("-{body}") } else { body }
}

fn string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            ' '..='\x7f' => out.push(c),
            _ => {
                let mut b = [0u16; 2];
                for u in c.encode_utf16(&mut b) {
                    out.push_str(&format!("\\u{u:04x}"));
                }
            }
        }
    }
    out.push('"');
}

fn write(v: &Value, style: Style, depth: usize, out: &mut String) {
    let (open_sep, item_sep, key_sep) = match style {
        Style::Indent(n) => (format!("\n{}", " ".repeat(n * (depth + 1))), ",", ": "),
        Style::Compact => (String::new(), ",", ":"),
    };
    let close = match style {
        Style::Indent(n) => format!("\n{}", " ".repeat(n * depth)),
        Style::Compact => String::new(),
    };
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(x) => match x.as_f64() {
            Some(f) if x.is_f64() => out.push_str(&float(f)),
            _ => out.push_str(&x.to_string()),
        },
        Value::String(s) => string(s, out),
        Value::Array(a) if a.is_empty() => out.push_str("[]"),
        Value::Object(o) if o.is_empty() => out.push_str("{}"),
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push_str(item_sep);
                }
                out.push_str(&open_sep);
                write(x, style, depth + 1, out);
            }
            out.push_str(&close);
            out.push(']');
        }
        Value::Object(o) => {
            out.push('{');
            for (i, (k, x)) in o.iter().enumerate() {
                if i > 0 {
                    out.push_str(item_sep);
                }
                out.push_str(&open_sep);
                string(k, out);
                out.push_str(key_sep);
                write(x, style, depth + 1, out);
            }
            out.push_str(&close);
            out.push('}');
        }
    }
}

pub fn dumps(v: &Value, style: Style) -> String {
    let mut s = String::new();
    write(v, style, 0, &mut s);
    s
}

/// Text of a JSON file in UTF-8 (optional BOM) or UTF-16 (BOM or detected), then parsed.
pub fn loads(b: &[u8]) -> Result<Value, String> {
    let text = if b.starts_with(&[0xef, 0xbb, 0xbf]) {
        String::from_utf8(b[3..].to_vec()).map_err(|e| e.to_string())?
    } else if b.starts_with(&[0xff, 0xfe]) || (b.len() > 1 && b[1] == 0 && b[0] != 0) {
        utf16(b.strip_prefix(&[0xff, 0xfe]).unwrap_or(b), u16::from_le_bytes)?
    } else if b.starts_with(&[0xfe, 0xff]) || (b.len() > 1 && b[0] == 0 && b[1] != 0) {
        utf16(b.strip_prefix(&[0xfe, 0xff]).unwrap_or(b), u16::from_be_bytes)?
    } else {
        String::from_utf8(b.to_vec()).map_err(|e| e.to_string())?
    };
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn utf16(b: &[u8], f: fn([u8; 2]) -> u16) -> Result<String, String> {
    let u: Vec<u16> = b.chunks_exact(2).map(|c| f([c[0], c[1]])).collect();
    String::from_utf16(&u).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_float_repr() {
        for (x, s) in [(0.1, "0.1"), (1.0, "1.0"), (1e-5, "1e-05"), (0.0001, "0.0001"),
                       (1e16, "1e+16"), (1234567890123456.0, "1234567890123456.0"),
                       (8.850009180605412e-05, "8.850009180605412e-05"), (-2.5, "-2.5"),
                       (1.5e300, "1.5e+300"), (123.456, "123.456")] {
            assert_eq!(float(x), s);
        }
    }

    #[test]
    fn python_layout() {
        let v = json!({"a": [1, 2.0, "é"], "b": {}, "c": []});
        assert_eq!(dumps(&v, Style::Indent(1)),
                   "{\n \"a\": [\n  1,\n  2.0,\n  \"\\u00e9\"\n ],\n \"b\": {},\n \"c\": []\n}");
        assert_eq!(dumps(&v, Style::Compact), "{\"a\":[1,2.0,\"\\u00e9\"],\"b\":{},\"c\":[]}");
    }

    #[test]
    fn reads_bom_and_utf16() {
        let s = "{\"V\": [\"a\"]}";
        let mut b = vec![0xef, 0xbb, 0xbf];
        b.extend(s.as_bytes());
        assert!(loads(&b).is_ok());
        let mut u = vec![0xff, 0xfe];
        u.extend(s.encode_utf16().flat_map(|c| c.to_le_bytes()));
        assert_eq!(loads(&u).unwrap(), loads(s.as_bytes()).unwrap());
    }
}
