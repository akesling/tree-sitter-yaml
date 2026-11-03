//! This crate provides YAML language support for the [tree-sitter] parsing library.
//!
//! Typically, you will use the [`LANGUAGE`] constant to add this language to a
//! tree-sitter [`Parser`], and then use the parser to parse some code:
//!
//! ```
//! let code = r#"
//! key: value
//! list:
//!     - item1
//!     - item2
//! "#;
//! let mut parser = tree_sitter::Parser::new();
//! let language = tree_sitter_yaml::LANGUAGE;
//! parser
//!     .set_language(&language.into())
//!     .expect("Error loading YAML parser");
//! let tree = parser.parse(code, None).unwrap();
//! assert!(!tree.root_node().has_error());
//! ```
//!
//! [`Parser`]: https://docs.rs/tree-sitter/0.25.4/tree_sitter/struct.Parser.html
//! [tree-sitter]: https://tree-sitter.github.io/

use tree_sitter_language::LanguageFn;

extern "C" {
    fn tree_sitter_yaml() -> *const ();
}

/// The tree-sitter [`LanguageFn`] for this grammar.
pub const LANGUAGE: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_yaml) };

/// The content of the [`node-types.json`] file for this grammar.
///
/// [`node-types.json`]: https://tree-sitter.github.io/tree-sitter/using-parsers/6-static-node-types
pub const NODE_TYPES: &str = include_str!("../../src/node-types.json");

/// The highlight queries for this grammar.
pub const HIGHLIGHTS_QUERY: &str = include_str!("../../queries/highlights.scm");

#[cfg(test)]
mod tests {
    fn parse(text: &str) -> tree_sitter::Tree {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE.into())
            .expect("Error loading YAML parser");
        parser.parse(text, None).unwrap()
    }

    #[test]
    fn test_can_load_grammar() {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&super::LANGUAGE.into())
            .expect("Error loading YAML parser");
    }

    /// Rows past `i16::MAX` (#49): the scanner's rows used to wrap, and the
    /// whole document became one ERROR.
    #[test]
    fn test_rows_past_i16() {
        let lines = 40_000;
        let text: String = (0..lines).map(|i| format!("k{i}: v\n")).collect();
        let tree = parse(&text);
        let root = tree.root_node();
        assert!(
            !root.has_error(),
            "{}",
            root.to_sexp().get(..200).unwrap_or("")
        );
        assert_eq!(root.end_position().row, lines);
    }

    /// An implicit key that starts at a property (an anchor or a tag) on a
    /// row past `i16::MAX`: the key's column is the property's, which the
    /// scanner keeps only while the row it noted for the key is the
    /// current one.
    #[test]
    fn test_implicit_key_rows_past_i16() {
        let mut text: String = (0..40_000).map(|i| format!("k{i}: v\n")).collect();
        text.push_str("&a k:\n  x: y\n!t z: w\n");
        let tree = parse(&text);
        assert!(!tree.root_node().has_error());
    }

    /// Nesting deeper than the scanner's state fits in tree-sitter's
    /// 1,024-byte serialization buffer (#48). Before the bounds check this
    /// wrote past the buffer and aborted on tree-sitter's assertion; now the
    /// state is cut short and the parse goes on. Nesting that fits parses
    /// cleanly.
    #[test]
    fn test_nesting_past_the_serialization_buffer() {
        const FITS: usize = 250;
        let shapes: [fn(usize) -> String; 3] = [
            |depth| {
                (0..depth)
                    .map(|i| format!("{}k{i}:\n", " ".repeat(i)))
                    .collect::<String>()
                    + &" ".repeat(depth)
                    + "v\n"
            },
            |depth| {
                (0..depth)
                    .map(|i| format!("{}-\n", " ".repeat(2 * i)))
                    .collect::<String>()
                    + &" ".repeat(2 * depth)
                    + "x\n"
            },
            |depth| "- ".repeat(depth) + "x\n",
        ];
        for shape in shapes {
            for depth in [FITS, 251, 252, 253, 254, 255, 256, 300, 1_000] {
                let tree = parse(&shape(depth));
                if depth <= FITS {
                    assert!(!tree.root_node().has_error(), "depth {depth}");
                }
            }
        }
    }
}
