use std::rc::Rc;

fn purust_json_quote(value: &str) -> String {
    let units = purust_core::purust_string_to_utf16(value);
    let mut output = String::from("\"");
    let mut i = 0;
    while i < units.len() {
        let unit = units[i];
        match unit {
            0x22 => output.push_str("\\\""),
            0x5c => output.push_str("\\\\"),
            8 => output.push_str("\\b"),
            9 => output.push_str("\\t"),
            10 => output.push_str("\\n"),
            12 => output.push_str("\\f"),
            13 => output.push_str("\\r"),
            0..=0x1f => output.push_str(&format!("\\u{:04x}", unit)),
            0xd800..=0xdbff if units.get(i + 1).is_some_and(|v| (0xdc00..=0xdfff).contains(v)) => {
                output.push(purust_core::purust_char_from_code_unit(unit));
                i += 1;
                output.push(purust_core::purust_char_from_code_unit(units[i]));
            }
            0xd800..=0xdfff => output.push_str(&format!("\\u{:04x}", unit)),
            _ => output.push(purust_core::purust_char_from_code_unit(unit)),
        }
        i += 1;
    }
    output.push('"');
    output
}

fn purust_json_fields(fields: Vec<(String, crate::UnknownType)>) -> String {
    let parts: Vec<_> = fields.into_iter().map(|(key, value)| {
        format!("{}:{}", purust_json_quote(&key), Yoga_JSON__unsafeStringify(value))
    }).collect();
    format!("{{{}}}", parts.join(","))
}

pub fn Yoga_JSON__unsafeStringify(value: crate::UnknownType) -> String {
    match value.resolve() {
        crate::Value::Int(n) => n.to_string(),
        crate::Value::Number(n) if !n.is_finite() => "null".into(),
        crate::Value::Number(n) => ryu_js::Buffer::new().format_finite(*n).to_owned(),
        crate::Value::Bool(b) => b.to_string(),
        crate::Value::String(s) => purust_json_quote(s),
        crate::Value::Char(c) => purust_json_quote(&c.to_string()),
        crate::Value::Array(values) => format!("[{}]", values.iter().cloned()
            .map(Yoga_JSON__unsafeStringify).collect::<Vec<_>>().join(",")),
        crate::Value::Class(native) => {
            if let Some(object) = native.downcast_ref::<Rc<Purs_Foreign_Object::Object>>() {
                purust_json_fields(object.entries())
            } else {
                panic!("Yoga.JSON: unsupported opaque native value");
            }
        }
        _ => match value.__purust_record_fields() {
            Some(fields) => purust_json_fields(fields.entries()),
            None => panic!("Yoga.JSON: value has no qualified JSON representation"),
        },
    }
}
