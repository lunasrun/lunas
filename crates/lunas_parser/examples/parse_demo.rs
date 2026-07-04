//! End-to-end smoke check: parse a realistic `.lunas` file and print a summary.
//! Run with `cargo run -p lunas_parser --example parse_demo`.

use lunas_parser::parse;

fn main() {
    let source = "\
@input(optional)
count: number = 0

@use()
Counter from \"./Counter\"

html:
    <div class=\"app\">
        <h1>Hello {{ count }}</h1>
        <Counter value={count} />
        <br>
        <!-- a comment -->
    </div>

style:
    .app { display: flex; }

script:
    let count: number = 0
    const inc = (): void => { count++ }
";

    let (file, diagnostics) = parse(source);

    println!("directives: {}", file.directives.len());
    for d in &file.directives {
        println!("  {:?}", d);
    }

    if let Some(html) = &file.html {
        println!("html dom kind: {:?}", html.dom.kind);
        println!("html top-level nodes: {}", html.dom.children.len());
        println!("template top-level nodes: {}", html.template.nodes.len());
    }

    if let Some(script) = &file.script {
        println!("script text: {:?}", script.source.text);
    }

    println!("diagnostics: {}", diagnostics.len());
    for diag in &diagnostics {
        println!("{}", diag.render(source, &file.line_index));
    }
}
