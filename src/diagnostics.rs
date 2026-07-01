//! Diagnostics for Thymeleaf attributes.
//!
//! Operates purely on a parsed [`Document`]:
//!  * flags `th:*` / `data-th-*` attributes that are not part of the known
//!    Standard Dialect catalog (WARNING), and
//!  * runs a lightweight, low-false-positive bracket/paren balance check on the
//!    values of *known* Thymeleaf attributes (ERROR).

use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Range};

use crate::{document::Document, thymeleaf};

const SOURCE: &str = "thymeleaf_ls";

/// Computes diagnostics for a single document.
pub fn diagnostics(document: &Document) -> Vec<Diagnostic> {
    let mut out = Vec::new();

    for attr in document.attributes() {
        if !is_thymeleaf_name(&attr.name) {
            continue;
        }

        match thymeleaf::lookup(&attr.name) {
            None => {
                // Unknown Thymeleaf attribute -> warning at the name range.
                let range = to_range(document, attr.name_range.start, attr.name_range.end);
                out.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::WARNING),
                    source: Some(SOURCE.to_string()),
                    message: format!(
                        "Unknown Thymeleaf attribute '{}'",
                        canonical_display(&attr.name)
                    ),
                    ..Default::default()
                });
            }
            Some(_) => {
                // Known attribute: sanity-check the expression value.
                if let Some(msg) = unbalanced_message(&attr.value) {
                    let range =
                        to_range(document, attr.value_range.start, attr.value_range.end);
                    out.push(Diagnostic {
                        range,
                        severity: Some(DiagnosticSeverity::ERROR),
                        source: Some(SOURCE.to_string()),
                        message: msg,
                        ..Default::default()
                    });
                }
            }
        }
    }

    out
}

/// Whether `name` looks like a Thymeleaf attribute (`th:*` or `data-th-*`),
/// case-insensitively.
fn is_thymeleaf_name(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    lower.starts_with("th:") || lower.starts_with("data-th-")
}

/// Renders the attribute name in canonical `th:xxx` form for messages, while
/// preserving readability. Falls back to the verbatim name if it does not match
/// a recognizable form.
fn canonical_display(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("data-th-") {
        format!("th:{}", rest)
    } else if let Some(rest) = lower.strip_prefix("th:") {
        format!("th:{}", rest)
    } else {
        name.to_string()
    }
}

/// Converts a byte range into an LSP [`Range`].
fn to_range(document: &Document, start: usize, end: usize) -> Range {
    Range {
        start: document.position_at(start),
        end: document.position_at(end),
    }
}

/// Performs a conservative balance check over a Thymeleaf expression value.
///
/// Returns `Some(message)` only when brackets/parentheses are *clearly*
/// unbalanced. To avoid false positives on legitimate content (strings that may
/// contain stray brackets, apostrophes, etc.) the scanner:
///  * skips characters inside single- or double-quoted string literals, and
///  * tracks `(` `)`, `[` `]` and `{` `}` with a type-aware stack.
///
/// A closing bracket that does not match the most recent opener, or any leftover
/// unclosed opener at end-of-input, is reported.
fn unbalanced_message(value: &str) -> Option<String> {
    let mut stack: Vec<char> = Vec::new();
    let mut in_single = false;
    let mut in_double = false;

    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        // String-literal handling. Backslash escapes the next char inside a
        // quoted literal so an escaped quote does not toggle the state.
        if in_single {
            if c == '\\' {
                chars.next();
            } else if c == '\'' {
                in_single = false;
            }
            continue;
        }
        if in_double {
            if c == '\\' {
                chars.next();
            } else if c == '"' {
                in_double = false;
            }
            continue;
        }

        match c {
            '\'' => in_single = true,
            '"' => in_double = true,
            '(' | '[' | '{' => stack.push(c),
            ')' | ']' | '}' => {
                let expected = match c {
                    ')' => '(',
                    ']' => '[',
                    _ => '{',
                };
                match stack.pop() {
                    Some(open) if open == expected => {}
                    _ => {
                        // Either nothing open, or the wrong opener on top.
                        return Some(format!(
                            "Unbalanced expression: unexpected '{}'",
                            c
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    // If we ended inside a string literal we do not treat that as a bracket
    // imbalance (too likely to be a benign partial edit); only report leftover
    // openers.
    if let Some(open) = stack.first() {
        let close = match open {
            '(' => ')',
            '[' => ']',
            _ => '}',
        };
        return Some(format!("Unbalanced expression: missing '{}'", close));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    fn doc(s: &str) -> Document {
        Document::new(s.to_string())
    }

    #[test]
    fn valid_thymeleaf_attr_is_not_flagged() {
        let d = doc("<p th:text=\"${user.name}\">x</p>");
        let diags = diagnostics(&d);
        assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
    }

    #[test]
    fn unknown_thymeleaf_attr_is_flagged() {
        let d = doc("<p th:bogus=\"x\">y</p>");
        let diags = diagnostics(&d);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Some(DiagnosticSeverity::WARNING));
        assert!(diags[0].message.contains("th:bogus"));
    }

    #[test]
    fn plain_html_attrs_are_ignored() {
        let d = doc("<div class=\"box\" id=\"main\">x</div>");
        assert!(diagnostics(&d).is_empty());
    }

    #[test]
    fn unbalanced_expression_is_flagged() {
        let d = doc("<p th:text=\"${a\">x</p>");
        let diags = diagnostics(&d);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Some(DiagnosticSeverity::ERROR));
    }
}
