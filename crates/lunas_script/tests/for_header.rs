//! Tests for the `for` loop header parser.

use lunas_script::{parse_for, ForKind, ParsedFor};

macro_rules! ok_tests {
    ($($name:ident: $input:expr => ($kind:expr, $binding:expr, $iterable:expr)),+ $(,)?) => {
        $(#[test]
        fn $name() {
            let pf = parse_for($input)
                .unwrap_or_else(|| panic!("expected Some for input '{}'", $input));
            let exp = ParsedFor { kind: $kind, binding: $binding.into(), iterable: $iterable.into() };
            assert_eq!(pf, exp, "input: '{}'", $input);
        })+
    };
}

macro_rules! err_tests {
    ($($name:ident: $input:expr),+ $(,)?) => {
        $(#[test]
        fn $name() {
            assert!(parse_for($input).is_none(), "expected None for input '{}'", $input);
        })+
    };
}

ok_tests! {
    object_entries_1: "const [index, value] of Object.entries(data)" => (ForKind::Of, "[index, value]", "Object.entries(data)"),
    object_entries_2: "var { k , v } of Object.entries( myMap )" => (ForKind::Of, "{ k , v }", "Object.entries( myMap )"),
    method_entries_1: "const [idx, val] of myData.entries()" => (ForKind::Of, "[idx, val]", "myData.entries()"),
    method_entries_2: "let [ k, v ] of another_obj.get_items().entries()" => (ForKind::Of, "[ k, v ]", "another_obj.get_items().entries()"),
    method_entries_3: "[i, b] of bools.entries()" => (ForKind::Of, "[i, b]", "bools.entries()"),
    plain_of_1: "let value of dataArr" => (ForKind::Of, "value", "dataArr"),
    plain_of_2: "item of getItems()" => (ForKind::Of, "item", "getItems()"),
    plain_of_3: "val of obj.prop" => (ForKind::Of, "val", "obj.prop"),
    // `Object.entries(ident)` keeps the `.entries()` call (a bare ident
    // arg is not unwrapped), matching `object_entries_1`.
    whitespace_1: " [ i , v ] of Object.entries(  sampleData ) " => (ForKind::Of, "[ i , v ]", "Object.entries(  sampleData )"),
    whitespace_2: "const\t[ index , value ]\rof\t myArr.entries( \n ) " => (ForKind::Of, "[ index , value ]", "myArr.entries( \n )"),
    whitespace_3: " let \t item \n of \t data " => (ForKind::Of, "item", "data"),
    whitespace_4: " key\tin\tobject " => (ForKind::In, "key", "object"),
    fn_rhs_1: "item of filteredItems()" => (ForKind::Of, "item", "filteredItems()"),
    edge_1: "let [ k, v ] of (getObj()).items.entries()" => (ForKind::Of, "[ k, v ]", "(getObj()).items"),
    edge_2: "const [idx, val] of Object.entries(await getData().then(r => r.json()))" => (ForKind::Of, "[idx, val]", "await getData().then(r => r.json())"),
    edge_3: "[i6] of [...Array(counts[5]).keys()]" => (ForKind::Of, "[i6]", "[...Array(counts[5]).keys()]"),
    edge_4: "i of [...Array(bools.length).keys()]" => (ForKind::Of, "i", "[...Array(bools.length).keys()]"),
    no_decl_array: "[a,b,c] of d" => (ForKind::Of, "[a,b,c]", "d"),
    no_decl_object: "const {i, v} of nonEntries()" => (ForKind::Of, "{i, v}", "nonEntries()"),
    trailing_comma: "const [a,] of d" => (ForKind::Of, "[a,]", "d"),
    for_in_destructuring: "const [i, v] in data.entries()" => (ForKind::In, "[i, v]", "data.entries()"),
    plain_in: "key in obj" => (ForKind::In, "key", "obj"),
    of_keyword_literal: "x of [1, 2, 3]" => (ForKind::Of, "x", "[1, 2, 3]"),
}

err_tests! {
    invalid_1: "for foo bar",
    invalid_2: "let [a] of",
    invalid_extra_tokens: "let a in obj extra",
    invalid_4: "let [a,b c] of data",
    invalid_6: "x of y z",
    invalid_7: "in obj",
    invalid_8: "let x y z of arr",
    invalid_empty: "",
    invalid_13: "val of obj.",
    invalid_14: "val of obj.()",
    invalid_plain_for: "let i = 0; i < 10; i++",
}
