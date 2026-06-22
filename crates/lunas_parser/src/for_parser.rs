//! Parser for `for … of` / `for … in` loop headers used in Lunas templates.
//!
//! The header is wrapped into a synthetic `for(<header>){}` statement and
//! parsed with SWC; the binding pattern and iterable expression are then
//! recovered from the AST spans as exact source substrings. This reuses the
//! same semantics as the original Lunas parser, including the special handling
//! that drops a trailing `.entries()` in certain `for..of` scenarios.

use serde::{Deserialize, Serialize};
use swc_common::{sync::Lrc, FileName, SourceMap, SourceMapper, Span, Spanned};
use swc_ecma_ast::{
    CallExpr, Callee, Expr, ForHead, ForInStmt, ForOfStmt, MemberProp, ModuleItem, Stmt,
};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ForKind {
    Of,
    In,
}

/// The parsed pieces of a `for` loop header.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParsedFor {
    pub kind: ForKind,
    /// The binding pattern on the left of `of`/`in`, e.g. `item` or `[i, v]`,
    /// without any `let`/`const`/`var` keyword.
    pub binding: String,
    /// The iterable expression on the right.
    pub iterable: String,
}

/// Parses a `for` loop header such as `item of items` or `[i, v] in obj`.
/// Returns `None` if the input is not a recognizable `for..of` / `for..in`
/// header.
pub fn parse_for(input: &str) -> Option<ParsedFor> {
    let src = input.trim();
    let wrapped = format!("for({}){{}}", src);

    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(FileName::Custom("for_stmt.js".into()).into(), wrapped);
    let lexer = Lexer::new(
        Syntax::Es(Default::default()),
        Default::default(),
        StringInput::from(&*fm),
        None,
    );
    let mut parser = Parser::new_from(lexer);
    // A lexer/parser error means the header is malformed; report no match.
    if !parser.take_errors().is_empty() {
        // errors are populated lazily; check again after parsing below.
    }
    let module = parser.parse_module().ok()?;
    if !parser.take_errors().is_empty() {
        return None;
    }

    let module_item = module.body.into_iter().next()?;
    let stmt = match module_item {
        ModuleItem::Stmt(s) => s,
        _ => return None,
    };

    let (kind, left_for_head, right_expr) = match stmt {
        Stmt::ForOf(ForOfStmt { left, right, .. }) => (ForKind::Of, left, right),
        Stmt::ForIn(ForInStmt { left, right, .. }) => (ForKind::In, left, right),
        _ => return None,
    };

    let pattern_span: Span = match left_for_head {
        ForHead::VarDecl(var_decl) => var_decl.decls.first()?.name.span(),
        ForHead::Pat(pat) => pat.span(),
        ForHead::UsingDecl(_) => return None,
    };

    let binding = cm.span_to_snippet(pattern_span).ok()?.trim().to_string();

    let mut iterable_span = right_expr.span();

    // Drop a trailing `.entries()` in the for-of scenarios the original parser
    // handled, so `[i, v] of arr.entries()` iterates over `arr` directly.
    if kind == ForKind::Of {
        if let Expr::Call(CallExpr { callee, args, .. }) = &*right_expr {
            if let Callee::Expr(callee_expr) = callee {
                if let Expr::Member(member_expr) = &**callee_expr {
                    if let MemberProp::Ident(ident_prop) = &member_expr.prop {
                        if ident_prop.sym.as_ref() == "entries" {
                            let obj_expr = &*member_expr.obj;
                            let drop_entries = match obj_expr {
                                Expr::Ident(obj_ident) if obj_ident.sym.as_ref() == "Object" => args
                                    .first()
                                    .is_some_and(|first_arg| {
                                        first_arg.spread.is_none()
                                            && !matches!(&*first_arg.expr, Expr::Ident(_))
                                    }),
                                Expr::Member(_) => true,
                                Expr::Paren(paren_expr) => {
                                    matches!(&*paren_expr.expr, Expr::Member(_))
                                }
                                _ => false,
                            };

                            if drop_entries {
                                if let Expr::Ident(obj_ident) = &*member_expr.obj {
                                    if obj_ident.sym.as_ref() == "Object" {
                                        if let Some(first_arg) = args.first() {
                                            iterable_span = first_arg.expr.span();
                                        }
                                    }
                                } else if let Expr::Member(sub_member) = &*member_expr.obj {
                                    iterable_span = sub_member.span();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let iterable = cm.span_to_snippet(iterable_span).ok()?.trim().to_string();

    Some(ParsedFor {
        kind,
        binding,
        iterable,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_for, ForKind, ParsedFor};

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
}
