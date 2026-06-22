//! The HTML parser must never panic, for any input. These tests throw a large
//! deterministic corpus of adversarial and pseudo-random strings at it and
//! assert each call returns (a panic would unwind and be caught here).

use lunas_html_parser::parse_html;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn assert_no_panic(input: &str) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let _ = parse_html(input);
    }));
    assert!(result.is_ok(), "parse_html panicked on input: {:?}", input);
}

#[test]
fn adversarial_literals_do_not_panic() {
    let cases = [
        "",
        "<",
        ">",
        "</",
        "<>",
        "</>",
        "<!",
        "<!-",
        "<!--",
        "<!---",
        "<!-->",
        "<![CDATA[",
        "<?xml?>",
        "<a",
        "<a ",
        "<a =",
        "<a b=",
        "<a b=\"",
        "<a b='",
        "<a/",
        "<a//>",
        "</a b=c>",
        "<a><b><c>",
        "</a></b></c>",
        "<script>",
        "<script><<<",
        "<style>}}}{{{",
        "<title>",
        "<textarea>",
        "<!-- <!-- -->",
        "<DIV></DIV>",
        "<a\0b>",
        "<\u{1F600}>",
        "<a b=\"\u{1F600}\">",
        "あ<div>い</div>う",
        "<a b c d e f g>",
        "&amp;&lt;&#x1F600;",
        "<a b=\"x\"y=\"z\">",
        "<<<<<<>>>>>>",
        "\0\0\0",
        "\r\n\r\n",
    ];
    for c in cases {
        assert_no_panic(c);
    }
}

#[test]
fn deeply_unbalanced_does_not_panic() {
    let opens: String = "<div>".repeat(2000);
    let closes: String = "</span>".repeat(2000);
    assert_no_panic(&opens);
    assert_no_panic(&closes);
    assert_no_panic(&format!("{opens}{closes}"));
}

/// A tiny deterministic LCG so the fuzz corpus is reproducible.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
}

#[test]
fn pseudo_random_fuzz_does_not_panic() {
    // Alphabet weighted toward characters that drive the lexer/parser states.
    let alphabet: &[char] = &[
        '<',
        '>',
        '/',
        '=',
        '"',
        '\'',
        '!',
        '-',
        '?',
        ':',
        '@',
        '$',
        '{',
        '}',
        ' ',
        '\n',
        '\t',
        '\r',
        '\0',
        'a',
        'b',
        'c',
        'x',
        '&',
        ';',
        '#',
        'あ',
        '\u{1F600}',
    ];
    let mut rng = Lcg(0x1234_5678_9abc_def0);
    for _ in 0..5000 {
        let len = (rng.next() % 40) as usize;
        let mut s = String::with_capacity(len);
        for _ in 0..len {
            let idx = (rng.next() as usize) % alphabet.len();
            s.push(alphabet[idx]);
        }
        assert_no_panic(&s);
    }
}
