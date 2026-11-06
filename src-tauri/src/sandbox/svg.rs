use quick_xml::events::{BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer, XmlVersion};
use std::collections::HashSet;

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const LIMIT: f64 = 100_000.0;

pub struct SvgInfo {
    pub svg: String,
    pub ids: HashSet<String>,
}

/// Validates the Phase 1 subset. viewBox units remain physical units (0.1 mm each).
/// Rebuild XML from decoded, validated values so no untrusted markup is forwarded.
pub fn validate_svg(source: &str) -> Result<SvgInfo, String> {
    if source.len() > 256 * 1024 {
        return Err("SVG source exceeds 256 KiB".into());
    }
    check_chars(source)?;
    let mut reader = Reader::from_str(source);
    reader.config_mut().enable_all_checks(true);
    reader.config_mut().allow_unmatched_ends = false;
    reader.config_mut().allow_dangling_amp = false;
    let mut writer = Writer::new(Vec::new());
    let mut ids = HashSet::new();
    let mut stack: Vec<String> = Vec::new();
    let mut count = 0;
    loop {
        let event = reader
            .read_event()
            .map_err(|e| format!("invalid SVG XML: {e}"))?;
        match event {
            Event::Start(ref start) | Event::Empty(ref start) => {
                if stack.is_empty() && count != 0 {
                    return Err("SVG must have exactly one root element".into());
                }
                count += 1;
                if count > 2048 {
                    return Err("SVG exceeds 2048 elements".into());
                }
                if stack.len() >= 32 {
                    return Err("SVG nesting exceeds 32 elements".into());
                }
                if stack
                    .last()
                    .is_some_and(|s| matches!(s.as_str(), "text" | "title" | "desc"))
                {
                    return Err("SVG text elements cannot contain child elements".into());
                }
                let clean = sanitize_start(start, stack.is_empty(), &mut ids)?;
                if matches!(event, Event::Start(_)) {
                    stack.push(clean.name().0.to_owned());
                    write(&mut writer, Event::Start(clean))?;
                } else {
                    write(&mut writer, Event::Empty(clean))?;
                }
            }
            Event::End(end) => {
                if stack.pop().as_deref() != Some(end.name().0) {
                    return Err("SVG closing tag does not match opening tag".into());
                }
                write(&mut writer, Event::End(end))?;
            }
            Event::Text(text) => {
                let value = text.xml10_content();
                if value.contains("]]>") {
                    return Err("invalid SVG text: unescaped ]]>".into());
                }
                if stack.is_empty() && !value.bytes().all(xml_space) {
                    return Err("non-whitespace text outside SVG root".into());
                }
                write(&mut writer, Event::Text(BytesText::new(&value)))?;
            }
            Event::GeneralRef(reference) => {
                if stack.is_empty() {
                    return Err("entity reference outside SVG root".into());
                }
                let raw = format!("&{};", reference.as_ref());
                let value =
                    quick_xml::escape::unescape_with(&raw, quick_xml::escape::resolve_xml_entity)
                        .map_err(|e| format!("invalid SVG text reference: {e}"))?;
                check_chars(&value)?;
                write(&mut writer, Event::Text(BytesText::new(&value)))?;
            }
            Event::CData(text) => {
                if stack.is_empty() {
                    return Err("CDATA outside SVG root".into());
                }
                write(
                    &mut writer,
                    Event::Text(BytesText::new(&text.xml10_content())),
                )?;
            }
            Event::Comment(_) => {} // Comments have no visual meaning; never forward them.
            Event::Decl(_) | Event::PI(_) | Event::DocType(_) => {
                return Err(
                    "SVG declarations, DTD/entities and processing instructions are forbidden"
                        .into(),
                );
            }
            Event::Eof => break,
        }
    }
    if count == 0 || !stack.is_empty() {
        return Err("SVG root is missing or incomplete".into());
    }
    let svg =
        String::from_utf8(writer.into_inner()).map_err(|e| format!("SVG output encoding: {e}"))?;
    Ok(SvgInfo { svg, ids })
}

fn write(writer: &mut Writer<Vec<u8>>, event: Event<'_>) -> Result<(), String> {
    writer
        .write_event(event)
        .map_err(|e| format!("cannot serialize safe SVG: {e}"))
}

fn check_chars(value: &str) -> Result<(), String> {
    if value.chars().all(|c| matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')) {
        Ok(())
    } else {
        Err("SVG contains an invalid XML character".into())
    }
}

fn sanitize_start(
    start: &BytesStart<'_>,
    root: bool,
    ids: &mut HashSet<String>,
) -> Result<BytesStart<'static>, String> {
    let name = start.name().0;
    if !matches!(
        name,
        "svg"
            | "g"
            | "rect"
            | "circle"
            | "ellipse"
            | "line"
            | "polyline"
            | "polygon"
            | "path"
            | "text"
            | "title"
            | "desc"
    ) {
        return Err(format!("SVG element <{name}> is not allowed"));
    }
    if root != (name == "svg") {
        return Err("SVG requires a single, non-nested <svg> root".into());
    }
    // quick-xml accepts adjacent quoted attributes; XML requires whitespace.
    let raw = start.attributes_raw().as_bytes();
    let mut quote = None;
    for (index, &byte) in raw.iter().enumerate() {
        if quote == Some(byte) {
            quote = None;
            if raw.get(index + 1).is_some_and(|b| !xml_space(*b)) {
                return Err("SVG attributes require whitespace separators".into());
            }
        } else if quote.is_none() && matches!(byte, b'\'' | b'"') {
            quote = Some(byte);
        }
    }
    let mut clean = BytesStart::new(name.to_owned());
    let (mut namespace, mut viewbox) = (false, false);
    for attribute in start.attributes().with_checks(true) {
        let attribute = attribute.map_err(|e| format!("invalid SVG attribute: {e}"))?;
        let key = attribute.key.0;
        let value = attribute
            .normalized_value_with(
                XmlVersion::Implicit1_0,
                1,
                quick_xml::escape::resolve_xml_entity,
            )
            .map_err(|e| format!("invalid SVG attribute {key}: {e}"))?;
        check_chars(&value)?;
        let valid = match key {
            "xmlns" if root => {
                namespace = value == SVG_NS;
                namespace
            }
            "viewBox" if root => {
                let values =
                    numbers(&value, false).map_err(|e| format!("invalid SVG viewBox: {e}"))?;
                viewbox = values.len() == 4 && values[2] > 0.0 && values[3] > 0.0;
                viewbox
            }
            "id" => {
                let valid = !value.is_empty()
                    && value.len() <= 64
                    && value.as_bytes()[0].is_ascii_alphabetic()
                    && value
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'));
                if valid && !ids.insert(value.to_string()) {
                    return Err(format!("duplicate ID in SVG: {value}"));
                }
                valid
            }
            "fill" | "stroke" => paint(&value),
            "opacity" | "fill-opacity" | "stroke-opacity" => scalar(&value, 0.0, 1.0),
            "stroke-width" => scalar(&value, 0.0, LIMIT),
            "width" | "height" if matches!(name, "svg" | "rect") => {
                scalar(&value, 0.0, LIMIT) && (!root || scalar(&value, f64::MIN_POSITIVE, LIMIT))
            }
            "x" | "y" if matches!(name, "rect" | "text") => scalar(&value, -LIMIT, LIMIT),
            "cx" | "cy" if matches!(name, "circle" | "ellipse") => scalar(&value, -LIMIT, LIMIT),
            "r" if name == "circle" => scalar(&value, 0.0, LIMIT),
            "rx" | "ry" if matches!(name, "rect" | "ellipse") => scalar(&value, 0.0, LIMIT),
            "x1" | "y1" | "x2" | "y2" if name == "line" => scalar(&value, -LIMIT, LIMIT),
            "points" if matches!(name, "polygon" | "polyline") => {
                numbers(&value, false).is_ok_and(|v| {
                    v.len() % 2 == 0 && v.len() >= if name == "polygon" { 6 } else { 4 }
                })
            }
            "d" if name == "path" => safe_path(&value),
            "font-size" if name == "text" => scalar(&value, 0.0, LIMIT),
            "font-family" if name == "text" => {
                matches!(value.as_ref(), "serif" | "sans-serif" | "monospace")
            }
            "text-anchor" if name == "text" => matches!(value.as_ref(), "start" | "middle" | "end"),
            _ => return Err(format!("SVG attribute {key} is not allowed on <{name}>")),
        };
        if !valid {
            return Err(format!("invalid SVG attribute {key} on <{name}>"));
        }
        clean.push_attribute((key, value.as_ref()));
    }
    if root && (!namespace || !viewbox) {
        return Err(
            "SVG root requires xmlns=\"http://www.w3.org/2000/svg\" and a valid viewBox".into(),
        );
    }
    Ok(clean)
}

fn paint(value: &str) -> bool {
    matches!(value, "none" | "currentColor")
        || value.strip_prefix('#').is_some_and(|hex| {
            matches!(hex.len(), 3 | 4 | 6 | 8) && hex.bytes().all(|b| b.is_ascii_hexdigit())
        })
}

fn scalar(value: &str, min: f64, max: f64) -> bool {
    numbers(value, false).is_ok_and(|v| v.len() == 1 && v[0] >= min && v[0] <= max)
}

fn xml_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n')
}

// Parse SVG decimal numbers, not Rust's broader float grammar (NaN/inf/units).
fn numbers(value: &str, compact: bool) -> Result<Vec<f64>, String> {
    let bytes = value.as_bytes();
    let mut i = 0;
    let mut values = Vec::new();
    while i < bytes.len() {
        let before = i;
        while i < bytes.len() && xml_space(bytes[i]) {
            i += 1;
        }
        let mut separated = i > before;
        if i == bytes.len() {
            break;
        }
        if bytes[i] == b',' {
            if values.is_empty() {
                return Err("leading comma in numeric list".into());
            }
            separated = true;
            i += 1;
            while i < bytes.len() && xml_space(bytes[i]) {
                i += 1;
            }
        }
        if !values.is_empty() && !separated && !compact {
            return Err("numbers require whitespace or comma separators".into());
        }
        let start = i;
        if i < bytes.len() && matches!(bytes[i], b'+' | b'-') {
            i += 1;
        }
        let mut digits = 0;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
            digits += 1;
        }
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
                digits += 1;
            }
        }
        if digits == 0 {
            return Err("expected finite SVG number".into());
        }
        if i < bytes.len() && matches!(bytes[i], b'e' | b'E') {
            i += 1;
            if i < bytes.len() && matches!(bytes[i], b'+' | b'-') {
                i += 1;
            }
            let exponent = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i == exponent {
                return Err("invalid numeric exponent".into());
            }
        }
        let number = value[start..i]
            .parse::<f64>()
            .map_err(|_| "invalid SVG number")?;
        if !number.is_finite() || number.abs() > LIMIT {
            return Err("SVG number must be finite with magnitude <= 100000".into());
        }
        values.push(number);
    }
    Ok(values)
}

fn safe_path(value: &str) -> bool {
    if value.len() > 16 * 1024 {
        return false;
    }
    let value = value.trim_matches(|c: char| c.is_ascii_whitespace());
    if !value.starts_with(['M', 'm']) {
        return false;
    }
    let mut rest = value;
    while !rest.is_empty() {
        let command = rest.as_bytes()[0];
        let arity = match command.to_ascii_uppercase() {
            b'M' | b'L' | b'T' => 2,
            b'H' | b'V' => 1,
            b'C' => 6,
            b'S' | b'Q' => 4,
            b'A' => 7,
            b'Z' => 0,
            _ => return false,
        };
        rest = &rest[1..];
        let end = rest
            .bytes()
            .position(|b| b.is_ascii_alphabetic() && !matches!(b, b'e' | b'E'))
            .unwrap_or(rest.len());
        let Ok(values) = numbers(&rest[..end], true) else {
            return false;
        };
        if arity == 0 {
            if !values.is_empty() {
                return false;
            }
        } else if values.is_empty() || values.len() % arity != 0 {
            return false;
        }
        if command.eq_ignore_ascii_case(&b'A')
            && values.as_chunks::<7>().0.iter().any(|v| {
                v[0] < 0.0 || v[1] < 0.0 || !matches!(v[3], 0.0 | 1.0) || !matches!(v[4], 0.0 | 1.0)
            })
        {
            return false;
        }
        rest = &rest[end..];
    }
    true
}

#[cfg(test)]
#[path = "svg_tests.rs"]
mod tests;
