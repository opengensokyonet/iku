use super::*;
use crate::{BareItem, Integer, Key, Token};
use alloc::string::String;
use bytes::Bytes;

fn parse(input: &'static str) -> (Result<InnerList, SyntaxError>, Bytes) {
    let mut parser = Parser::new(Bytes::from_static(input.as_bytes()));
    let result = InnerList::parse(&mut parser);
    (result, Bytes::copy_from_slice(parser.rest()))
}

fn ok(input: &'static str) -> InnerList {
    parse(input).0.unwrap()
}

fn err(input: &'static str) -> SyntaxError {
    parse(input).0.unwrap_err()
}

fn serialized(list: &InnerList) -> String {
    let mut out = Vec::new();
    list.serialize(&mut out);
    String::from_utf8(out).unwrap()
}

fn canonical(input: &'static str) -> String {
    serialized(&ok(input))
}

fn int_item(value: i64) -> Item {
    Item::new(BareItem::Integer(Integer::new(value).unwrap()))
}

#[test]
fn parses_items() {
    let list = ok("(1 2 3)");
    assert_eq!(list.items(), [int_item(1), int_item(2), int_item(3)]);
    assert!(list.parameters().is_empty());
    let list = ok("(\"foo\" \"bar\")");
    assert_eq!(
        list.items(),
        [
            Item::new(BareItem::String(crate::String::from_static("foo").unwrap())),
            Item::new(BareItem::String(crate::String::from_static("bar").unwrap())),
        ]
    );
    let list = ok("(abc_123)");
    assert_eq!(
        list.items(),
        [Item::new(BareItem::Token(
            Token::from_static("abc_123").unwrap()
        ))]
    );
}

#[test]
fn parses_empty_lists() {
    assert!(ok("()").items().is_empty());
    assert!(ok("( )").items().is_empty());
    assert!(ok("(   )").items().is_empty());
    assert_eq!(canonical("(  )"), "()");
}

#[test]
fn extra_spaces_are_skipped() {
    assert_eq!(canonical("(  a   b  )"), "(a b)");
    assert_eq!(canonical("( 1 )"), "(1)");
}

#[test]
fn items_keep_their_parameters() {
    let list = ok("(\"foo\"; a=1;b=2 \"bar\")");
    assert_eq!(list.items().len(), 2);
    assert_eq!(list.items()[0].parameters().len(), 2);
    assert!(list.items()[1].parameters().is_empty());
    assert_eq!(
        canonical("(\"foo\"; a=1;b=2 \"bar\")"),
        "(\"foo\";a=1;b=2 \"bar\")"
    );
    assert_eq!(canonical("(a;x b;y=2)"), "(a;x b;y=2)");
}

#[test]
fn the_inner_list_has_parameters_of_its_own() {
    let list = ok("(a b);a=1;b");
    assert_eq!(list.items().len(), 2);
    assert_eq!(list.parameters().len(), 2);
    assert_eq!(
        list.parameters().get("a"),
        Some(&BareItem::Integer(Integer::new(1).unwrap()))
    );
    assert_eq!(list.parameters().get("b"), Some(&BareItem::Boolean(true)));
    assert_eq!(canonical("(a;x);y=1"), "(a;x);y=1");
    assert_eq!(
        canonical("(\"foo\" \"bar\");lvl=5"),
        "(\"foo\" \"bar\");lvl=5"
    );
    assert_eq!(canonical("();a"), "();a");
}

#[test]
fn parse_stops_after_the_parameters() {
    let (result, rest) = parse("(a);x=1, b");
    result.unwrap();
    assert_eq!(&rest[..], b", b");
    let (result, rest) = parse("(a)b");
    result.unwrap();
    assert_eq!(&rest[..], b"b");
    let (result, rest) = parse("(a) ;x");
    result.unwrap();
    assert_eq!(&rest[..], b" ;x");
}

#[test]
fn unclosed_list_reports_the_opening_parenthesis() {
    assert_eq!(err("("), SyntaxError::UnclosedInnerList { index: 0 });
    assert_eq!(err("(a"), SyntaxError::UnclosedInnerList { index: 0 });
    assert_eq!(err("(a b"), SyntaxError::UnclosedInnerList { index: 0 });
    assert_eq!(err("(a "), SyntaxError::UnclosedInnerList { index: 0 });
    assert_eq!(err("(  "), SyntaxError::UnclosedInnerList { index: 0 });
    assert_eq!(err("(a;x=1"), SyntaxError::UnclosedInnerList { index: 0 });
}

#[test]
fn unclosed_list_index_counts_from_the_line_start() {
    let mut parser = Parser::new(Bytes::from_static(b"x, (a"));
    parser.skip(3);
    assert_eq!(
        InnerList::parse(&mut parser),
        Err(SyntaxError::UnclosedInnerList { index: 3 })
    );
}

#[test]
fn items_are_followed_by_a_space_or_a_parenthesis() {
    assert_eq!(
        err("(a,b)"),
        SyntaxError::ExpectedInnerListDelimiter { index: 2 }
    );
    assert_eq!(
        err("(a\"b\")"),
        SyntaxError::ExpectedInnerListDelimiter { index: 2 }
    );
    assert_eq!(
        err("(a\tb)"),
        SyntaxError::ExpectedInnerListDelimiter { index: 2 }
    );
    assert_eq!(
        err("(1;a=2,3)"),
        SyntaxError::ExpectedInnerListDelimiter { index: 6 }
    );
    assert_eq!(
        err("(a=1)"),
        SyntaxError::ExpectedInnerListDelimiter { index: 2 }
    );
    assert_eq!(err("(a b;x=1;)"), SyntaxError::InvalidKeyStart { index: 9 });
}

#[test]
fn syntax_errors_of_the_items_pass_through() {
    assert_eq!(err("((a))"), SyntaxError::InvalidBareItemStart { index: 1 });
    assert_eq!(err("(,)"), SyntaxError::InvalidBareItemStart { index: 1 });
    assert_eq!(err("(a;)"), SyntaxError::InvalidKeyStart { index: 3 });
    assert_eq!(err("(1.2345)"), SyntaxError::FractionTooLong { index: 6 });
    assert_eq!(err("(a);"), SyntaxError::InvalidKeyStart { index: 4 });
}

#[test]
fn constructors_and_accessors() {
    let mut parameters = Parameters::new();
    parameters.insert(Key::from_static("p").unwrap(), BareItem::Boolean(false));
    let list =
        InnerList::with_parameters(alloc::vec![int_item(1), int_item(2)], parameters.clone());
    assert_eq!(list.items(), [int_item(1), int_item(2)]);
    assert_eq!(list.parameters(), &parameters);
    assert_eq!(serialized(&list), "(1 2);p=?0");
    assert_eq!(
        list.clone().into_parts(),
        (alloc::vec![int_item(1), int_item(2)], parameters)
    );

    let list = InnerList::new(alloc::vec![int_item(5)]);
    assert!(list.parameters().is_empty());
    assert_eq!(serialized(&list), "(5)");
    assert_eq!(serialized(&InnerList::default()), "()");
    assert_eq!(InnerList::default(), InnerList::new(Vec::new()));
}

#[test]
fn rfc_examples_are_canonical() {
    for text in [
        "(\"foo\" \"bar\")",
        "(\"baz\")",
        "(\"bat\" \"one\")",
        "()",
        "(\"foo\";a=1;b=2);lvl=5",
        "(abc_123);a=1;b=2",
    ] {
        assert_eq!(canonical(text), text);
    }
}
