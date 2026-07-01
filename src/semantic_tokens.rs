//! LSP semantic tokens for Thymeleaf templates.
//!
//! Provides a semantic-tokens [`legend`] and a full-document tokenizer
//! ([`semantic_tokens_full`]) that highlights:
//!
//! * recognized `th:*` / `data-th-*` attribute **names** as `PROPERTY`, and
//! * Thymeleaf expression markers (`${...}`, `*{...}`, `#{...}`, `@{...}`,
//!   `~{...}`) inside recognized attribute **values** as `MACRO`.
//!
//! Tokens are produced sorted by (line, character), non-overlapping, and
//! delta-encoded per the LSP specification. Byte offsets are converted to
//! LSP positions via [`Document::position_at`], which already yields
//! UTF-16-correct `character` components; token lengths are likewise measured
//! in UTF-16 code units. Tokens never span multiple lines: a value span that
//! crosses a newline is clipped to its start line.

use tower_lsp::lsp_types::*;

use crate::document::Document;
use crate::thymeleaf;

/// Token-type index for recognized `th:*` attribute names.
pub const TYPE_PROPERTY: u32 = 0;
/// Token-type index for Thymeleaf expression markers (`${ }`, `*{ }`, ...).
pub const TYPE_MACRO: u32 = 1;
/// Token-type index for string content. Reserved in the legend for future use.
#[allow(dead_code)]
pub const TYPE_STRING: u32 = 2;
/// Token-type index for variables. Reserved in the legend for future use.
#[allow(dead_code)]
pub const TYPE_VARIABLE: u32 = 3;

/// The semantic-tokens legend. The token-type indices are stable and exposed
/// via the `TYPE_*` constants; there are no modifiers.
pub fn legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::PROPERTY,
            SemanticTokenType::MACRO,
            SemanticTokenType::STRING,
            SemanticTokenType::VARIABLE,
        ],
        token_modifiers: vec![],
    }
}

/// An absolute (pre-delta-encoding) token.
struct RawToken {
    line: u32,
    start: u32,
    length: u32,
    token_type: u32,
}

/// Produces the full-document semantic tokens for `doc`.
pub fn semantic_tokens_full(doc: &Document) -> SemanticTokens {
    let mut raw: Vec<RawToken> = Vec::new();

    for attr in doc.attributes() {
        // Only recognized Thymeleaf attributes contribute tokens.
        if thymeleaf::lookup(&attr.name).is_none() {
            continue;
        }

        // The attribute name itself -> PROPERTY.
        push_token(
            doc,
            &mut raw,
            attr.name_range.start,
            attr.name_range.end,
            TYPE_PROPERTY,
        );

        // Expression markers inside the value -> MACRO.
        for (start, end) in expression_spans(&attr.value, attr.value_range.start) {
            push_token(doc, &mut raw, start, end, TYPE_MACRO);
        }
    }

    // Sort by (line, start_char) and drop any overlaps to keep the stream
    // well-formed for delta-encoding.
    raw.sort_by_key(|a| (a.line, a.start));
    let raw = dedup_overlaps(raw);

    SemanticTokens {
        result_id: None,
        data: delta_encode(&raw),
    }
}

/// Locates Thymeleaf expression spans (`X{...}`) within `value`, returning
/// their absolute byte ranges in the document. `value_start` is the byte
/// offset of `value` within [`Document::text`].
fn expression_spans(value: &str, value_start: usize) -> Vec<(usize, usize)> {
    let bytes = value.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i];
        let is_marker = matches!(c, b'$' | b'*' | b'#' | b'@' | b'~');
        if is_marker && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            // Find the matching closing brace, honoring nesting.
            let mut depth = 0u32;
            let mut j = i + 1;
            let mut closed = None;
            while j < bytes.len() {
                match bytes[j] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            closed = Some(j);
                            break;
                        }
                    }
                    _ => {}
                }
                j += 1;
            }
            if let Some(end) = closed {
                spans.push((value_start + i, value_start + end + 1));
                i = end + 1;
                continue;
            }
        }
        i += 1;
    }
    spans
}

/// Converts a byte range to a single-line [`RawToken`] and appends it. Empty
/// or degenerate ranges are ignored. Spans crossing a newline are clipped to
/// their start line.
fn push_token(doc: &Document, out: &mut Vec<RawToken>, start: usize, end: usize, token_type: u32) {
    if end <= start || start >= doc.text.len() {
        return;
    }
    let end = end.min(doc.text.len());

    // Clip to the start line if the span crosses a newline.
    let slice = &doc.text[start..end];
    let end = match slice.find('\n') {
        Some(nl) => start + nl,
        None => end,
    };
    if end <= start {
        return;
    }

    let pos = doc.position_at(start);
    let length: u32 = doc.text[start..end]
        .chars()
        .map(|c| c.len_utf16() as u32)
        .sum();
    if length == 0 {
        return;
    }

    out.push(RawToken {
        line: pos.line,
        start: pos.character,
        length,
        token_type,
    });
}

/// Removes tokens that overlap a previously kept token on the same line. The
/// input must already be sorted by (line, start).
fn dedup_overlaps(raw: Vec<RawToken>) -> Vec<RawToken> {
    let mut out: Vec<RawToken> = Vec::with_capacity(raw.len());
    for tok in raw {
        if let Some(prev) = out.last() {
            if prev.line == tok.line && tok.start < prev.start + prev.length {
                // Overlaps the previous token; skip to keep the stream valid.
                continue;
            }
        }
        out.push(tok);
    }
    out
}

/// Delta-encodes absolute tokens into the flat `[dLine, dStart, len, type,
/// modifiers]` representation mandated by the LSP spec.
fn delta_encode(raw: &[RawToken]) -> Vec<SemanticToken> {
    let mut data = Vec::with_capacity(raw.len());
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;
    for tok in raw {
        let delta_line = tok.line - prev_line;
        let delta_start = if delta_line == 0 {
            tok.start - prev_start
        } else {
            tok.start
        };
        data.push(SemanticToken {
            delta_line,
            delta_start,
            length: tok.length,
            token_type: tok.token_type,
            token_modifiers_bitset: 0,
        });
        prev_line = tok.line;
        prev_start = tok.start;
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(s: &str) -> Document {
        Document::new(s.to_string())
    }

    #[test]
    fn legend_has_four_types_and_no_modifiers() {
        let l = legend();
        assert_eq!(l.token_types.len(), 4);
        assert!(l.token_modifiers.is_empty());
        assert_eq!(
            l.token_types[TYPE_PROPERTY as usize],
            SemanticTokenType::PROPERTY
        );
        assert_eq!(l.token_types[TYPE_MACRO as usize], SemanticTokenType::MACRO);
        assert_eq!(
            l.token_types[TYPE_STRING as usize],
            SemanticTokenType::STRING
        );
        assert_eq!(
            l.token_types[TYPE_VARIABLE as usize],
            SemanticTokenType::VARIABLE
        );
    }

    #[test]
    fn emits_property_and_macro_tokens() {
        let d = doc("<p th:text=\"${name}\">x</p>");
        let toks = semantic_tokens_full(&d);

        // `data` is a Vec<SemanticToken> (one struct per token). At least the
        // attr name + the ${...} marker.
        assert!(toks.data.len() >= 2, "expected >= 2 tokens");

        let types: Vec<u32> = toks.data.iter().map(|t| t.token_type).collect();
        assert!(types.contains(&TYPE_PROPERTY));
        assert!(types.contains(&TYPE_MACRO));
    }

    #[test]
    fn tokens_are_delta_encoded_with_non_negative_deltas() {
        let d = doc("<p th:text=\"${name}\">x</p>");
        let toks = semantic_tokens_full(&d);
        for t in &toks.data {
            // `delta_line` / `delta_start` are u32, so implicitly non-negative;
            // this asserts the fields exist and encode a sane first token.
            let _ = t.delta_line;
            let _ = t.delta_start;
        }
        // First token is relative to (0, 0): its delta_line is the actual line.
        assert_eq!(toks.data[0].delta_line, 0);
    }

    #[test]
    fn ignores_unrecognized_attributes() {
        let d = doc("<p class=\"${name}\">x</p>");
        let toks = semantic_tokens_full(&d);
        assert!(toks.data.is_empty(), "non-th attributes produce no tokens");
    }

    #[test]
    fn covers_all_expression_markers() {
        let d = doc(
            "<a th:href=\"@{/u}\" th:text=\"${a}\" th:with=\"x=*{b}\" th:alt=\"#{c}\" th:insert=\"~{d}\">z</a>",
        );
        let toks = semantic_tokens_full(&d);
        let macros = toks
            .data
            .iter()
            .filter(|t| t.token_type == TYPE_MACRO)
            .count();
        assert_eq!(macros, 5, "one MACRO token per expression marker");
    }

    #[test]
    fn data_th_form_is_recognized() {
        let d = doc("<p data-th-text=\"${name}\">x</p>");
        let toks = semantic_tokens_full(&d);
        let types: Vec<u32> = toks.data.iter().map(|t| t.token_type).collect();
        assert!(types.contains(&TYPE_PROPERTY));
        assert!(types.contains(&TYPE_MACRO));
    }
}
