use crate::value::Value;

pub fn to_ron(value: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, value, 0);
    out.push('\n');
    out
}

fn write_value(out: &mut String, v: &Value, depth: usize) {
    match v {
        Value::Int(i) => out.push_str(&i.to_string()),
        Value::Float(f) => out.push_str(&format!("{:?}", f)),
        Value::Str(s) => write_string(out, s),
        Value::Bool(b) => out.push_str(&b.to_string()),
        Value::DateTime(d) => write_string(out, d),
        Value::Array(arr) => write_list(out, arr, depth),
        Value::Table(table) => write_map(out, table, depth),
    }
}
fn write_list(out: &mut String, items: &[Value], depth: usize) {
    if items.is_empty() {
        out.push_str("[]");
        return;
    }
    if items.iter().all(|v| match v {
        Value::Table(_) => false,
        Value::Array(_) => false,
        _ => true,
    }) {
        out.push_str("[");
        for (i, item) in items.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            write_value(out, item, depth);
        }
        out.push_str("]");
        return;
    }
    out.push_str("[\n");
    for item in items {
        indent(out, depth + 1);
        write_value(out, item, depth + 1);
        out.push_str(",\n");
    }
    indent(out, depth);
    out.push_str("]");
}
fn write_map(out: &mut String, entries: &[(String, Value)], depth: usize) {
    if entries.is_empty() {
        out.push_str("{}");
        return;
    }
    out.push_str("{\n");
    for (k, v) in entries {
        indent(out, depth + 1);
        write_string(out, k);
        out.push_str(": ");
        write_value(out, v, depth + 1);
        out.push_str(",\n");
    }
    indent(out, depth);
    out.push_str("}");
}
fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{{{:x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}
fn indent(out: &mut String, depth: usize) {
    out.push_str(&"\t".repeat(depth));
}
