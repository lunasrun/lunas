//! Strips TypeScript syntax from a script block, producing plain JavaScript.
//!
//! Uses SWC's TypeScript transform. Unlike the original implementation this is
//! panic-free: parse failures are surfaced as an error string the caller turns
//! into a [`Diagnostic`](lunas_span::Diagnostic).

use swc_common::{
    comments::SingleThreadedComments, sync::Lrc, FileName, Globals, Mark, SourceMap, GLOBALS,
};
use swc_ecma_codegen::to_code_default;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_transforms_base::{fixer::fixer, hygiene::hygiene, resolver};
use swc_ecma_transforms_typescript::{typescript, Config};
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum TsToJsError {
    #[error("failed to parse TypeScript: {0}")]
    Parse(String),
}

/// Transforms TypeScript source into JavaScript by stripping type annotations
/// and TS-specific syntax.
pub(crate) fn transform_ts_to_js(ts_code: &str) -> Result<String, TsToJsError> {
    let cm: Lrc<SourceMap> = Default::default();
    let fm = cm.new_source_file(
        Lrc::new(FileName::Custom("input.ts".into())),
        ts_code.to_string(),
    );

    let comments = SingleThreadedComments::default();
    let lexer = Lexer::new(
        Syntax::Typescript(TsSyntax {
            tsx: false,
            ..Default::default()
        }),
        Default::default(),
        StringInput::from(&*fm),
        Some(&comments),
    );

    let mut parser = Parser::new_from(lexer);
    let module = parser
        .parse_program()
        .map_err(|e| TsToJsError::Parse(format!("{:?}", e)))?;

    let globals = Globals::default();
    let code = GLOBALS.set(&globals, || {
        let unresolved_mark = Mark::new();
        let top_level_mark = Mark::new();

        let module = module.apply(resolver(unresolved_mark, top_level_mark, true));
        let module = module.apply(typescript(
            Config {
                verbatim_module_syntax: true,
                no_empty_export: true,
                import_not_used_as_values: typescript::ImportsNotUsedAsValues::Preserve,
                ..Config::default()
            },
            unresolved_mark,
            top_level_mark,
        ));
        let module = module.apply(hygiene());
        let program = module.apply(fixer(Some(&comments)));
        to_code_default(cm, Some(&comments), &program)
    });

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_types_keeps_imports() {
        let ts = r#"
            import axios from 'axios';
            interface Args { name: string; }
            function greet(arg: Args): void {
                console.log(`Hello, ${arg.name}!`);
            }
        "#;
        let js = transform_ts_to_js(ts).expect("transform failed");
        assert!(js.contains("function greet("));
        assert!(!js.contains("string"));
        assert!(js.contains("import axios from 'axios';"));
    }

    #[test]
    fn empty_input_ok() {
        assert_eq!(transform_ts_to_js("").expect("ok").trim(), "");
    }

    #[test]
    fn strips_let_type_annotation() {
        let js = transform_ts_to_js("let count: number = 0").expect("ok");
        assert!(!js.contains("number"));
        assert!(js.contains("count"));
    }

    #[test]
    fn invalid_ts_is_error() {
        assert!(transform_ts_to_js("let x: = =").is_err());
    }
}
