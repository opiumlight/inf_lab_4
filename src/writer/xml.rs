use crate::value::Value;

pub fn to_xml(value: &Value) -> Result<String, String> {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    write_element(&mut out, "root", value, 0)?;
    Ok(out)
}

fn indent(out: &mut String, depth: usize) {
    for _ in 0..depth {
        out.push_str("  ");
    }
}

fn write_element(out: &mut String, key: &str, v: &Value, depth: usize) -> Result<(), String> {
    let (tag, attr) = if is_name(key) {
        (key, String::new())
    } else {
        ("entry", format!(" key=\"{}\"", escape(key, true)?))
    };
    indent(out, depth);
    match v {
        Value::Table(entries) if !entries.is_empty() => {
            out.push_str(&format!("<{tag}{attr}>\n"));
            for (k, val) in entries {
                write_element(out, k, val, depth + 1)?;
            }
            indent(out, depth);
            out.push_str(&format!("</{tag}>\n"));
        }
        Value::Array(items) if !items.is_empty() => {
            out.push_str(&format!("<{tag}{attr}>\n"));
            for item in items {
                write_element(out, "item", item, depth + 1)?;
            }
            indent(out, depth);
            out.push_str(&format!("</{tag}>\n"));
        }
        Value::Table(_) | Value::Array(_) => out.push_str(&format!("<{tag}{attr}/>\n")),
        scalar => {
            let text = match scalar {
                Value::Int(i) => i.to_string(),
                Value::Float(f) => format!("{:?}", f),
                Value::Bool(b) => b.to_string(),
                Value::Str(s) | Value::DateTime(s) => escape(s, false)?,
                _ => unreachable!(),
            };
            out.push_str(&format!("<{tag}{attr}>{text}</{tag}>\n"));
        }
    }
    Ok(())
}

fn is_name(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_alphabetic() || c == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
}

fn escape(s: &str, in_attr: bool) -> Result<String, String> {
    let mut res = String::new();
    for c in s.chars() {
        match c {
            '&' => res.push_str("&amp;"),
            '<' => res.push_str("&lt;"),
            '>' => res.push_str("&gt;"),
            '"' if in_attr => res.push_str("&quot;"),
            '\r' => res.push_str("&#13;"),
            '\n' | '\t' if in_attr => res.push_str(&format!("&#{};", c as u32)),
            '\n' | '\t' => res.push(c),
            c if (c as u32) < 0x20 || c == '\u{FFFE}' || c == '\u{FFFF}' => {
                return Err(format!("символ U+{:04X} недопустим в XML 1.0", c as u32));
            }
            c => res.push(c),
        }
    }
    Ok(res)
}
