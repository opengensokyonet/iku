//! The public test vectors of the HTTP working group,
//! <https://github.com/httpwg/structured-field-tests> (see
//! `tests/vectors/README.md`).
//!
//! Each vector is parsed with the type its `header_type` names and compared
//! with `expected`; when it parses, the result is serialized and compared
//! with `canonical`, or with the raw lines joined by `, `.

use bytes::Bytes;
use iku_semantics::field::{FieldName, FieldSection, FieldValue};
use iku_structured_fields::{BareItem, Dictionary, InnerList, Item, List, ListMember, Parameters};
use serde_json::{Value, json};

/// Vectors that this package does not pass yet, by file and name. Empty: all
/// of them pass.
const SKIPPED: &[(&str, &str)] = &[];

fn base32(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = String::new();
    for chunk in bytes.chunks(5) {
        let mut buffer = [0u8; 5];
        buffer[..chunk.len()].copy_from_slice(chunk);
        let bits = buffer
            .iter()
            .fold(0u64, |acc, &b| (acc << 8) | u64::from(b));
        let characters = (chunk.len() * 8).div_ceil(5);
        for i in 0..8 {
            if i < characters {
                out.push(char::from(ALPHABET[((bits >> (35 - 5 * i)) & 31) as usize]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn bare_item_json(bare: &BareItem) -> Value {
    match bare {
        BareItem::Integer(n) => json!(n.get()),
        BareItem::Decimal(d) => json!(d.thousandths() as f64 / 1000.0),
        BareItem::String(s) => json!(s.as_str()),
        BareItem::Token(t) => json!({"__type": "token", "value": t.as_str()}),
        BareItem::ByteSequence(b) => json!({"__type": "binary", "value": base32(b)}),
        BareItem::Boolean(b) => json!(b),
        BareItem::Date(d) => json!({"__type": "date", "value": d.seconds().get()}),
        BareItem::DisplayString(s) => json!({"__type": "displaystring", "value": s}),
    }
}

fn parameters_json(parameters: &Parameters) -> Value {
    Value::Array(
        parameters
            .iter()
            .map(|(key, value)| json!([key.as_str(), bare_item_json(value)]))
            .collect(),
    )
}

fn item_json(item: &Item) -> Value {
    json!([
        bare_item_json(item.bare_item()),
        parameters_json(item.parameters())
    ])
}

fn inner_list_json(list: &InnerList) -> Value {
    let items: Vec<Value> = list.items().iter().map(item_json).collect();
    json!([items, parameters_json(list.parameters())])
}

fn member_json(member: &ListMember) -> Value {
    match member {
        ListMember::Item(i) => item_json(i),
        ListMember::InnerList(l) => inner_list_json(l),
    }
}

/// What parsing and serializing a vector gave, as JSON and as a line.
fn run(header_type: &str, raw: &[String]) -> Result<(Value, Option<String>), String> {
    let name = FieldName::new(Bytes::from_static(b"x-test")).map_err(|e| e.to_string())?;
    let mut fields = FieldSection::new();
    for line in raw {
        // A field value never starts or ends with whitespace (RFC 9110
        // §5.5), so the parser never sees the spaces that a structured field
        // may start and end with (RFC 9651 §4.2). Strip those, as a field
        // line parser would; a leading or trailing HTAB is not allowed there
        // and makes the line not a field value, which counts as a failure.
        let value = FieldValue::new(Bytes::copy_from_slice(line.trim_matches(' ').as_bytes()))
            .map_err(|e| format!("not a field value: {e}"))?;
        fields.append(name.clone(), value);
    }
    let line = |value: FieldValue| String::from_utf8_lossy(value.as_bytes()).into_owned();
    match header_type {
        "item" => match Item::from_fields(&fields, &name).map_err(|e| e.to_string())? {
            None => Err("no line".into()),
            Some(parsed) => Ok((item_json(&parsed), Some(line(parsed.to_field_value())))),
        },
        "list" => {
            let parsed = List::from_fields(&fields, &name).map_err(|e| e.to_string())?;
            let parsed = parsed.unwrap_or_default();
            let json = Value::Array(parsed.as_slice().iter().map(member_json).collect());
            Ok((json, parsed.to_field_value().map(line)))
        }
        "dictionary" => {
            let parsed = Dictionary::from_fields(&fields, &name).map_err(|e| e.to_string())?;
            let parsed = parsed.unwrap_or_default();
            let json = Value::Array(
                parsed
                    .iter()
                    .map(|(key, value)| json!([key.as_str(), member_json(value)]))
                    .collect(),
            );
            Ok((json, parsed.to_field_value().map(line)))
        }
        other => panic!("header_type {other}"),
    }
}

/// Checks one vector; the error says what differs.
fn check(vector: &Value) -> Result<(), String> {
    let header_type = vector["header_type"].as_str().expect("header_type");
    let raw: Vec<String> = vector["raw"]
        .as_array()
        .expect("raw")
        .iter()
        .map(|line| line.as_str().expect("raw line").to_owned())
        .collect();
    let must_fail = vector["must_fail"].as_bool().unwrap_or(false);
    let can_fail = vector["can_fail"].as_bool().unwrap_or(false);
    let outcome = run(header_type, &raw);
    if must_fail {
        return match outcome {
            Err(_) => Ok(()),
            Ok((parsed, _)) => Err(format!("must fail, but parsed as {parsed}")),
        };
    }
    let (parsed, serialized) = match outcome {
        Ok(ok) => ok,
        Err(_) if can_fail => return Ok(()),
        Err(reason) => return Err(format!("failed to parse: {reason}")),
    };
    if parsed != vector["expected"] {
        return Err(format!(
            "parsed as {parsed}, expected {}",
            vector["expected"]
        ));
    }
    let want: Option<String> = match vector.get("canonical") {
        Some(canonical) => canonical
            .as_array()
            .expect("canonical")
            .first()
            .map(|s| s.as_str().expect("canonical line").to_owned()),
        None => Some(raw.join(", ")).filter(|line| !line.is_empty()),
    };
    // An item is never omitted.
    let got = serialized.filter(|line| !line.is_empty());
    if got != want {
        return Err(format!("serialized as {got:?}, expected {want:?}"));
    }
    Ok(())
}

fn run_file(file: &str, text: &str) {
    let vectors: Vec<Value> = serde_json::from_str(text).expect("a JSON array");
    let mut failures = Vec::new();
    let mut unexpected_passes = Vec::new();
    for vector in &vectors {
        let name = vector["name"].as_str().expect("name");
        let skipped = SKIPPED.contains(&(file, name));
        match (check(vector), skipped) {
            (Err(reason), false) => failures.push(format!("{name}: {reason}")),
            (Ok(()), true) => unexpected_passes.push(name.to_owned()),
            _ => {}
        }
    }
    assert!(
        failures.is_empty() && unexpected_passes.is_empty(),
        "{file}: {} of {} vectors fail:\n{}\nskipped but passing: {unexpected_passes:?}",
        failures.len(),
        vectors.len(),
        failures.join("\n")
    );
}

macro_rules! vector_files {
    ($($test:ident => $file:literal),* $(,)?) => {$(
        #[test]
        fn $test() {
            run_file($file, include_str!(concat!("vectors/", $file)));
        }
    )*};
}

vector_files! {
    vectors_binary => "binary.json",
    vectors_boolean => "boolean.json",
    vectors_date => "date.json",
    vectors_dictionary => "dictionary.json",
    vectors_display_string => "display-string.json",
    vectors_examples => "examples.json",
    vectors_item => "item.json",
    vectors_key_generated => "key-generated.json",
    vectors_list => "list.json",
    vectors_listlist => "listlist.json",
    vectors_number_generated => "number-generated.json",
    vectors_number => "number.json",
    vectors_param_dict => "param-dict.json",
    vectors_param_list => "param-list.json",
    vectors_param_listlist => "param-listlist.json",
    vectors_string_generated => "string-generated.json",
    vectors_string => "string.json",
    vectors_token_generated => "token-generated.json",
    vectors_token => "token.json",
}
