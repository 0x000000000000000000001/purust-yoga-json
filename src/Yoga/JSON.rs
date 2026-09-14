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
            0xd800..=0xdbff
                if units
                    .get(i + 1)
                    .is_some_and(|v| (0xdc00..=0xdfff).contains(v)) =>
            {
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
    let parts: Vec<_> = fields
        .into_iter()
        .filter(|(_, value)| !purust_json_omitted(value))
        .map(|(key, value)| {
            format!(
                "{}:{}",
                purust_json_quote(&key),
                Yoga_JSON__unsafeStringify(value)
            )
        })
        .collect();
    format!("{{{}}}", parts.join(","))
}

pub fn Yoga_JSON__unsafeStringify(value: crate::UnknownType) -> String {
    match value.resolve() {
        crate::Value::Null => "null".into(),
        crate::Value::Int(n) => n.to_string(),
        crate::Value::Number(n) if !n.is_finite() => "null".into(),
        crate::Value::Number(n) => ryu_js::Buffer::new().format_finite(*n).to_owned(),
        crate::Value::Bool(b) => b.to_string(),
        crate::Value::String(s) => purust_json_quote(s),
        crate::Value::Char(c) => purust_json_quote(&c.to_string()),
        crate::Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .cloned()
                .map(|value| if purust_json_omitted(&value) {
                    "null".into()
                } else {
                    Yoga_JSON__unsafeStringify(value)
                })
                .collect::<Vec<_>>()
                .join(",")
        ),
        crate::Value::Class(native) => {
            if let Some(object) = native.downcast_ref::<Rc<Purs_Foreign_Object::Object>>() {
                purust_json_fields(object.entries())
            } else if let Some(integer) = native.downcast_ref::<Rc<Purs_JS_BigInt::BigInt>>() {
                purust_json_quote(&integer.to_string())
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

fn purust_json_omitted(value: &crate::UnknownType) -> bool {
    // Unit is JS undefined, not JSON null. Native functions have no JSON value.
    matches!(
        value.resolve(),
        crate::Value::Unit
            | crate::Value::Func1(_)
            | crate::Value::Func2(_)
            | crate::Value::Func3(_)
            | crate::Value::Func4(_)
            | crate::Value::Func5(_)
            | crate::Value::Func6(_)
            | crate::Value::Func7(_)
            | crate::Value::Func8(_)
            | crate::Value::Func9(_)
            | crate::Value::Func10(_)
            | crate::Value::Func11(_)
            | crate::Value::Func12(_)
    )
}

pub fn Yoga_JSON__undefined() -> crate::UnknownType {
    crate::Value::Unit
}

pub fn Yoga_JSON__unsafePrettyStringify(spaces: i64, value: crate::UnknownType) -> String {
    // Formatting the qualified compact form keeps quoting and property order identical.
    let compact = Yoga_JSON__unsafeStringify(value);
    let gap = " ".repeat(spaces.clamp(0, 10) as usize);
    if gap.is_empty() {
        return compact;
    }
    let mut result = String::new();
    let mut depth = 0;
    let mut quoted = false;
    let mut escaped = false;
    let chars: Vec<_> = compact.chars().collect();
    for (index, &ch) in chars.iter().enumerate() {
        if quoted {
            result.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                quoted = false;
            }
            continue;
        }
        match ch {
            '"' => {
                quoted = true;
                result.push(ch);
            }
            '[' | '{' => {
                result.push(ch);
                depth += 1;
                if chars.get(index + 1) != Some(&if ch == '[' { ']' } else { '}' }) {
                    result.push('\n');
                    result.push_str(&gap.repeat(depth));
                }
            }
            ']' | '}' => {
                depth -= 1;
                if index > 0 && chars[index - 1] != if ch == ']' { '[' } else { '{' } {
                    result.push('\n');
                    result.push_str(&gap.repeat(depth));
                }
                result.push(ch);
            }
            ',' => {
                result.push_str(",\n");
                result.push_str(&gap.repeat(depth));
            }
            ':' => result.push_str(": "),
            _ => result.push(ch),
        }
    }
    result
}

fn purust_json_raise(name: &str, message: String) -> ! {
    Purs_Effect_Exception::purust_exception_raise(
        Purs_Effect_Exception::Effect_Exception_errorWithName(
            purust_core::purust_string_from_utf8(&message),
            name.into(),
        ),
    )
}

struct PurustJsonParser {
    units: Vec<u16>,
    position: usize,
}

enum PurustJsonFrame {
    Array(Vec<crate::UnknownType>),
    Object(purust_core::RecordFields, String),
}

impl PurustJsonParser {
    fn fail(&self, reason: &str) -> ! {
        purust_json_raise(
            "SyntaxError",
            format!("{} in JSON at position {}", reason, self.position),
        )
    }

    fn peek(&self) -> Option<u16> {
        self.units.get(self.position).copied()
    }

    fn whitespace(&mut self) {
        while matches!(self.peek(), Some(9 | 10 | 13 | 32)) {
            self.position += 1;
        }
    }

    fn take(&mut self, unit: u16) -> bool {
        if self.peek() == Some(unit) {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn string(&mut self) -> String {
        if !self.take(34) {
            self.fail("Expected double-quoted property name");
        }
        let mut result = String::new();
        loop {
            let Some(mut unit) = self.peek() else {
                self.fail("Unterminated string");
            };
            self.position += 1;
            match unit {
                34 => return result,
                0..=31 => self.fail("Bad control character in string literal"),
                92 => {
                    let Some(escape) = self.peek() else {
                        self.fail("Unterminated string");
                    };
                    self.position += 1;
                    unit = match escape {
                        34 | 47 | 92 => escape,
                        98 => 8,
                        102 => 12,
                        110 => 10,
                        114 => 13,
                        116 => 9,
                        117 => {
                            let mut code = 0;
                            for _ in 0..4 {
                                let digit = match self.peek() {
                                    Some(c @ 48..=57) => c - 48,
                                    Some(c @ 65..=70) => c - 55,
                                    Some(c @ 97..=102) => c - 87,
                                    _ => self.fail("Bad Unicode escape"),
                                };
                                self.position += 1;
                                code = code * 16 + digit;
                            }
                            code
                        }
                        _ => self.fail("Bad escaped character"),
                    };
                }
                _ => {}
            }
            // Never normalize a pair or replace a lone surrogate with U+FFFD.
            result.push(purust_core::purust_char_from_code_unit(unit));
        }
    }

    fn number(&mut self) -> crate::UnknownType {
        let start = self.position;
        self.take(45);
        if !self.take(48) {
            if !matches!(self.peek(), Some(49..=57)) {
                self.fail("Invalid number");
            }
            while matches!(self.peek(), Some(48..=57)) {
                self.position += 1;
            }
        }
        if self.take(46) {
            if !matches!(self.peek(), Some(48..=57)) {
                self.fail("Unterminated fractional number");
            }
            while matches!(self.peek(), Some(48..=57)) {
                self.position += 1;
            }
        }
        if matches!(self.peek(), Some(69 | 101)) {
            self.position += 1;
            if matches!(self.peek(), Some(43 | 45)) {
                self.position += 1;
            }
            if !matches!(self.peek(), Some(48..=57)) {
                self.fail("Exponent part is missing a number");
            }
            while matches!(self.peek(), Some(48..=57)) {
                self.position += 1;
            }
        }
        let spelling: String = self.units[start..self.position]
            .iter()
            .map(|&c| c as u8 as char)
            .collect();
        // JSON.parse uses IEEE-754 even for integers, including overflow and -0.
        crate::Value::Number(
            spelling
                .parse::<f64>()
                .unwrap_or_else(|_| self.fail("Invalid number")),
        )
    }

    fn property(&mut self) -> String {
        self.whitespace();
        let key = self.string();
        self.whitespace();
        if !self.take(58) {
            self.fail("Expected ':' after property name");
        }
        key
    }

    fn parse(mut self) -> crate::UnknownType {
        // An explicit stack avoids aborting the process on deeply nested input.
        let mut stack = Vec::new();
        loop {
            self.whitespace();
            let mut value = match self.peek() {
                Some(34) => crate::Value::String(self.string()),
                Some(45 | 48..=57) => self.number(),
                Some(110 | 116 | 102) => {
                    let (literal, value) = match self.peek() {
                        Some(110) => ("null", crate::Value::Null),
                        Some(116) => ("true", crate::Value::Bool(true)),
                        _ => ("false", crate::Value::Bool(false)),
                    };
                    for unit in literal.encode_utf16() {
                        if !self.take(unit) {
                            self.fail("Unexpected token");
                        }
                    }
                    value
                }
                Some(91) => {
                    self.position += 1;
                    self.whitespace();
                    if !self.take(93) {
                        stack.push(PurustJsonFrame::Array(Vec::new()));
                        continue;
                    }
                    crate::Value::Array(Rc::new(Vec::new()))
                }
                Some(123) => {
                    self.position += 1;
                    self.whitespace();
                    if !self.take(125) {
                        let key = self.property();
                        stack.push(PurustJsonFrame::Object(
                            purust_core::RecordFields::new(),
                            key,
                        ));
                        continue;
                    }
                    crate::Value::Class(Rc::new(Rc::new(Purs_Foreign_Object::Object::empty())))
                }
                None => self.fail("Unexpected end of JSON input"),
                _ => self.fail("Unexpected token"),
            };
            loop {
                self.whitespace();
                value = match stack.last_mut() {
                    None => {
                        if self.peek().is_some() {
                            self.fail("Unexpected non-whitespace character after JSON");
                        }
                        return value;
                    }
                    Some(PurustJsonFrame::Array(values)) => {
                        values.push(value);
                        if self.take(44) {
                            break;
                        }
                        if !self.take(93) {
                            self.fail("Expected ',' or ']' after array element");
                        }
                        let Some(PurustJsonFrame::Array(values)) = stack.pop() else {
                            unreachable!()
                        };
                        crate::Value::Array(Rc::new(values))
                    }
                    Some(PurustJsonFrame::Object(fields, key)) => {
                        fields.insert(std::mem::take(key), value);
                        if self.take(44) {
                            *key = self.property();
                            break;
                        }
                        if !self.take(125) {
                            self.fail("Expected ',' or '}' after property value");
                        }
                        let Some(PurustJsonFrame::Object(fields, _)) = stack.pop() else {
                            unreachable!()
                        };
                        crate::Value::Class(Rc::new(Rc::new(
                            Purs_Foreign_Object::Object::from_entries(fields.entries()),
                        )))
                    }
                };
            }
        }
    }
}

fn purust_json_bigint_whitespace(c: char) -> bool {
    matches!(purust_core::purust_char_to_code_unit(c), 0x9..=0xd | 0x20 | 0xa0 | 0x1680 | 0x2000..=0x200a | 0x2028 | 0x2029 | 0x202f | 0x205f | 0x3000 | 0xfeff)
}

fn purust_json_bigint_string(value: &crate::UnknownType) -> String {
    match value.resolve() {
        crate::Value::String(s) => s.clone(),
        crate::Value::Null | crate::Value::Unit => String::new(),
        crate::Value::Bool(b) => b.to_string(),
        crate::Value::Number(n) => ryu_js::Buffer::new().format(*n).into(),
        crate::Value::Array(values) => values
            .iter()
            .map(purust_json_bigint_string)
            .collect::<Vec<_>>()
            .join(","),
        crate::Value::Class(native) => {
            if let Some(integer) = native.downcast_ref::<Rc<Purs_JS_BigInt::BigInt>>() {
                return integer.to_string();
            }
            if let Some(object) = native.downcast_ref::<Rc<Purs_Foreign_Object::Object>>() {
                if object.get("toString").is_some() {
                    purust_json_raise(
                        "TypeError",
                        "Cannot convert object to primitive value".into(),
                    );
                }
            }
            "[object Object]".into()
        }
        _ => unreachable!("JSON reviver only receives parsed JSON values"),
    }
}

fn purust_json_bigint(value: crate::UnknownType) -> crate::UnknownType {
    let integer = match value.resolve() {
        crate::Value::Null => {
            purust_json_raise("TypeError", "Cannot convert null to a BigInt".into())
        }
        crate::Value::Bool(value) => Purs_JS_BigInt::BigInt::from(u8::from(*value)),
        crate::Value::Number(value) => {
            if !value.is_finite() || value.fract() != 0.0 {
                purust_json_raise(
                    "RangeError",
                    "The number cannot be converted to a BigInt because it is not an integer"
                        .into(),
                );
            }
            format!("{:.0}", value)
                .parse::<Purs_JS_BigInt::BigInt>()
                .expect("finite integral decimal")
        }
        _ => {
            let text = purust_json_bigint_string(&value);
            let text = text.trim_matches(purust_json_bigint_whitespace);
            if text.is_empty() {
                Purs_JS_BigInt::BigInt::from(0)
            } else {
                let (digits, radix) = if text.starts_with("0x") || text.starts_with("0X") {
                    (&text[2..], 16)
                } else if text.starts_with("0b") || text.starts_with("0B") {
                    (&text[2..], 2)
                } else if text.starts_with("0o") || text.starts_with("0O") {
                    (&text[2..], 8)
                } else {
                    (text, 10)
                };
                let unsigned = if radix == 10 {
                    digits.strip_prefix(['+', '-']).unwrap_or(digits)
                } else {
                    digits
                };
                if unsigned.is_empty()
                    || !unsigned.chars().all(|c| c.is_ascii() && c.is_digit(radix))
                {
                    purust_json_raise("SyntaxError", "Cannot convert string to a BigInt".into());
                }
                Purs_JS_BigInt::BigInt::parse_bytes(digits.as_bytes(), radix).unwrap_or_else(|| {
                    purust_json_raise("SyntaxError", "Cannot convert string to a BigInt".into())
                })
            }
        }
    };
    crate::Value::Class(Rc::new(Rc::new(integer)))
}

fn purust_json_revive(value: crate::UnknownType) -> crate::UnknownType {
    // Visit the final own properties: duplicate JSON keys are overwritten before
    // reviver invocation, just as JSON.parse does.
    enum Visit {
        Enter(String, crate::UnknownType),
        Array(String, usize),
        Object(String, Vec<String>),
    }
    let mut visits = vec![Visit::Enter(String::new(), value)];
    let mut results = Vec::new();
    while let Some(visit) = visits.pop() {
        let (key, value) = match visit {
            Visit::Enter(key, value) => match value.resolve() {
                crate::Value::Array(items) => {
                    visits.push(Visit::Array(key, items.len()));
                    for (index, item) in items.iter().enumerate().rev() {
                        visits.push(Visit::Enter(index.to_string(), item.clone()));
                    }
                    continue;
                }
                crate::Value::Class(native) => {
                    let object = native
                        .downcast_ref::<Rc<Purs_Foreign_Object::Object>>()
                        .expect("parsed JSON object");
                    let entries = object.entries();
                    visits.push(Visit::Object(
                        key,
                        entries.iter().map(|(key, _)| key.clone()).collect(),
                    ));
                    for (key, value) in entries.into_iter().rev() {
                        visits.push(Visit::Enter(key, value));
                    }
                    continue;
                }
                _ => (key, value),
            },
            Visit::Array(key, length) => {
                let items = results.split_off(results.len() - length);
                (key, crate::Value::Array(Rc::new(items)))
            }
            Visit::Object(key, keys) => {
                let items = results.split_off(results.len() - keys.len());
                (
                    key,
                    crate::Value::Class(Rc::new(Rc::new(
                        Purs_Foreign_Object::Object::from_entries(
                            keys.into_iter().zip(items).collect(),
                        ),
                    ))),
                )
            }
        };
        results.push(if key == "big" {
            purust_json_bigint(value)
        } else {
            value
        });
    }
    results.pop().expect("JSON root")
}

pub fn Yoga_JSON__parseJSON() -> crate::UnknownType {
    // EffectFn1 is a direct uncurried callback; runEffectFn1 supplies the thunk.
    crate::Value::Func1(purust_core::Func1::Static(|payload| {
        let value = PurustJsonParser {
            units: purust_core::purust_string_to_utf16(&payload.unwrap_string()),
            position: 0,
        }
        .parse();
        purust_json_revive(value)
    }))
}
