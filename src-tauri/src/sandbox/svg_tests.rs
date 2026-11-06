use super::validate_svg;

const OPEN: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">"#;

fn svg(body: &str) -> String {
    format!("{OPEN}{body}</svg>")
}

fn rejects(source: &str, reason: &str) {
    let error = validate_svg(source).err().expect("unsafe SVG was accepted");
    assert!(error.contains(reason), "expected {reason:?}, got {error:?}");
}

#[test]
fn valid_simple_led() {
    let source = svg(
        r##"<title>Simple LED</title><g fill="none" stroke="#333" stroke-width="0.5">
        <rect x="30" y="10" width="40" height="60" rx="4"/>
        <circle id="dyn-led" cx="50" cy="30" r="15" fill="#ff0000" opacity="0.5"/>
        <line x1="40" y1="70" x2="40" y2="95"/>
        <circle id="pin-anode" cx="40" cy="95" r="2"/>
        <circle id="pin-cathode" cx="65.4" cy="95" r="2"/>
        </g>"##,
    );
    let info = validate_svg(&source).expect("valid LED");
    assert_eq!(info.ids.len(), 3);
    assert!(info.ids.contains("dyn-led"));
    assert!(info.ids.contains("pin-anode"));
    assert!(info.svg.contains("25.4") || info.svg.contains("65.4"));
    assert_eq!(
        validate_svg(&info.svg).expect("normalized SVG").svg,
        info.svg
    );
}

#[test]
fn viewbox_and_namespace_are_mandatory() {
    rejects(r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#, "viewBox");
    rejects(r#"<svg viewBox="0 0 10 10"/>"#, "xmlns");
    for value in [
        "0 0 0 10",
        "0 0 -1 10",
        "0 0 10 0",
        "0 0 100001 10",
        "NaN 0 10 10",
        "0 0 inf 10",
        "0 0 1e999 10",
        "0 0 10",
        "0 0 10 10 10",
        "0,,0,10,10",
        "0 0 10px 10",
    ] {
        rejects(
            &format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{value}"/>"#),
            "viewBox",
        );
    }
    assert!(validate_svg(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="-10,-20,100000,1"/>"#
    )
    .is_ok());
}

#[test]
fn active_elements_and_attributes_are_forbidden() {
    for name in [
        "script",
        "style",
        "foreignObject",
        "image",
        "use",
        "a",
        "iframe",
        "animate",
        "filter",
        "unknown",
    ] {
        rejects(&svg(&format!("<{name}/>")), "element");
    }
    for attr in [
        "onload=\"alert(1)\"",
        "onclick=\"x\"",
        "style=\"fill:red\"",
        "href=\"https://evil.test/x\"",
        "transform=\"scale(2)\"",
        "class=\"evil\"",
        "data-x=\"x\"",
    ] {
        rejects(&svg(&format!("<rect {attr}/>")), "attribute");
    }
}

#[test]
fn dtd_entities_and_processing_instructions_are_forbidden() {
    for prefix in [
        r#"<!DOCTYPE svg SYSTEM "https://evil.test/x">"#,
        r#"<!DOCTYPE svg [<!ENTITY x SYSTEM "file:///etc/passwd">]>"#,
        "<?evil instruction?>",
        "<?xml version=\"1.0\"?>",
    ] {
        assert!(validate_svg(&format!("{prefix}{}", svg(""))).is_err());
    }
    rejects(&svg("<text>&unknown;</text>"), "reference");
    assert!(validate_svg(&svg("<text>&#0;</text>")).is_err());
    assert!(validate_svg(&svg("<text>&#xD800;</text>")).is_err());
}

#[test]
fn namespaces_cannot_be_rebound_or_prefixed() {
    for body in [
        r#"<g xmlns="http://evil.test"/>"#,
        r#"<g xmlns=""/>"#,
        r#"<g xmlns:x="http://www.w3.org/2000/svg"/>"#,
        r#"<x:rect/>"#,
        r##"<rect xlink:href="#x"/>"##,
        r#"<svg viewBox="0 0 1 1"/>"#,
    ] {
        assert!(validate_svg(&svg(body)).is_err(), "{body}");
    }
    rejects(
        r#"<svg xmlns="http://evil.test" viewBox="0 0 1 1"/>"#,
        "xmlns",
    );
    assert!(
        validate_svg(r#"<s:svg xmlns:s="http://www.w3.org/2000/svg" viewBox="0 0 1 1"/>"#).is_err()
    );
}

#[test]
fn decoded_attribute_values_must_pass_the_same_allowlist() {
    for paint in [
        "url(https://evil.test/x)",
        "u&#114;l(https://evil.test/x)",
        "&#117;rl(#x)",
        "&amp;#117;rl(#x)",
        "red",
        "var(--paint)",
        "#123;stroke:red",
    ] {
        rejects(&svg(&format!(r#"<rect fill="{paint}"/>"#)), "fill");
    }
    assert!(validate_svg(&svg(r##"<rect fill="&#35;f00" opacity="&#48;.5"/>"##)).is_ok());
    assert!(validate_svg(&svg(r#"<rect fill="&unknown;"/>"#)).is_err());
}

#[test]
fn duplicate_attributes_ids_and_invalid_ids_are_rejected() {
    rejects(&svg(r#"<rect x="1" x="2"/>"#), "attribute");
    rejects(
        &svg(r#"<rect id="same"/><circle id="s&#97;me"/>"#),
        "duplicate ID",
    );
    rejects(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1" viewBox="0 0 2 2"/>"#,
        "attribute",
    );
    for id in [
        "",
        "1pin",
        "-pin",
        "_pin",
        "a:b",
        "a b",
        "ümlaut",
        "a.b",
        &"a".repeat(65),
    ] {
        rejects(&svg(&format!(r#"<rect id="{id}"/>"#)), "id");
    }
    assert!(validate_svg(&svg(&format!(r#"<rect id="{}"/>"#, "a".repeat(64)))).is_ok());
}

#[test]
fn malformed_and_multiple_roots_are_rejected() {
    for source in [
        "",
        " ",
        "<svg",
        "</svg>",
        &format!("{OPEN}<g></svg>"),
        &format!("{OPEN}<g>"),
        &format!("{}{}", svg(""), svg("")),
        &format!("junk{}", svg("")),
        &format!("{}junk", svg("")),
        &svg("<rect x='1></rect>"),
        &svg("<rect x=1/>"),
        &svg("<rect x='1'y='2'/>"),
        &svg("<rect x='1' / >"),
        &svg("<g></g extra>"),
        &svg("<!-- bad -- comment -->"),
        &svg("<!-- bad --->"),
        &svg("<text>A & B</text>"),
        &svg("<text>bad ]]> text</text>"),
    ] {
        assert!(validate_svg(source).is_err(), "accepted {source:?}");
    }
    assert!(validate_svg(&format!(" \n{}\t", svg(""))).is_ok());
}

#[test]
fn comments_are_removed_and_character_references_preserve_text() {
    let source = svg("<!-- harmless --><text>&lt;&gt;&amp;&apos;&quot;&#10;</text>");
    let info = validate_svg(&source).expect("safe references and comment");
    assert!(!info.svg.contains("<!--"));
    assert!(info.svg.contains("&lt;&gt;&amp;&apos;&quot;\n"));
    assert!(validate_svg(&info.svg).is_ok());
    assert!(validate_svg(&format!("&#32;{}", svg(""))).is_err());
    assert!(validate_svg(&format!("<![CDATA[ ]]>{}", svg(""))).is_err());
}

#[test]
fn resource_limits_include_empty_elements_and_root() {
    rejects(&" ".repeat(256 * 1024 + 1), "256 KiB");
    let body = "<rect/>".repeat(2047);
    assert!(validate_svg(&svg(&body)).is_ok());
    rejects(&svg(&(body + "<rect/>")), "2048");
    let body = format!("{}{}", "<g>".repeat(31), "</g>".repeat(31));
    assert!(validate_svg(&svg(&body)).is_ok());
    let body = format!("{}<rect/>{}", "<g>".repeat(31), "</g>".repeat(31));
    rejects(&svg(&body), "32");
}

#[test]
fn geometry_and_opacity_are_finite_bounded_and_element_specific() {
    for body in [
        r#"<rect x="NaN"/>"#,
        r#"<rect y="-100001"/>"#,
        r#"<rect width="-1"/>"#,
        r#"<circle r="-1"/>"#,
        r#"<circle cx="inf"/>"#,
        r#"<ellipse ry="1e999"/>"#,
        r#"<line x1="100001"/>"#,
        r#"<rect opacity="-0.1"/>"#,
        r#"<rect opacity="1.1"/>"#,
        r#"<rect fill-opacity="NaN"/>"#,
        r#"<rect stroke-opacity="inf"/>"#,
        r#"<rect stroke-width="-1"/>"#,
        r#"<rect x="10px"/>"#,
        r#"<rect cx="10"/>"#,
        r#"<circle width="10"/>"#,
        r#"<polygon points="0 0 1"/>"#,
        r#"<polyline points="0 0 100001 0"/>"#,
    ] {
        assert!(validate_svg(&svg(body)).is_err(), "accepted {body}");
    }
    assert!(validate_svg(&svg(r#"<ellipse cx="-100000" cy="1e2" rx="0" ry="100000"/><polyline points="0,0 1,2"/><polygon points="0 0 1 0 1 1"/>"#)).is_ok());
}

#[test]
fn path_data_is_bounded_and_cannot_contain_urls_or_free_identifiers() {
    assert!(validate_svg(&svg(
        r#"<path d="M0 0L10-2.5h2v3C1 2 3 4 5 6s1 2 3 4Q1 2 3 4t1 2A2 3 0 0 1 5 6z"/>"#
    ))
    .is_ok());
    for path in [
        "url(http://evil.test)",
        "M0 0 LNaN 2",
        "M0 0L1e999 2",
        "M0 0 L100001 0",
        "M0 0 X1 2",
        "L0 0",
        "M0",
        "M0 0L",
        "M0 0A-1 2 0 0 1 3 4",
        "M0 0A1 2 0 2 1 3 4",
        "M0,,0",
    ] {
        assert!(
            validate_svg(&svg(&format!(r#"<path d="{path}"/>"#))).is_err(),
            "{path}"
        );
    }
    assert!(validate_svg(&svg(&format!(
        r#"<path d="M0 0{}"/>"#,
        " L0 0".repeat(4000)
    )))
    .is_err());
}

#[test]
fn escaped_text_is_normalized_without_becoming_markup() {
    let source = svg(
        r#"<text x="1" y="2" font-size="12" font-family="sans-serif" text-anchor="middle">A &amp; B &lt;script&gt; &#x1F4A1; &quot;ok&quot;</text><desc><![CDATA[<img onerror='x'>]]></desc>"#,
    );
    let info = validate_svg(&source).expect("harmless text");
    assert!(info.svg.contains("&lt;script&gt;"));
    assert!(info.svg.contains("&lt;img"));
    assert!(!info.svg.contains("<![CDATA["));
    assert!(info.svg.contains('💡'));
    assert!(validate_svg(&info.svg).is_ok());
    for body in [
        r#"<text font-family="url(x)">x</text>"#,
        r#"<text font-size="-1">x</text>"#,
        r#"<text text-anchor="url(x)">x</text>"#,
        "<text>\u{1}</text>",
    ] {
        assert!(validate_svg(&svg(body)).is_err());
    }
}
