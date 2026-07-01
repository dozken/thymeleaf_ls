//! Document highlight for Thymeleaf: highlights every occurrence, *within the
//! current document*, of the symbol under the cursor.
//!
//! Two symbol kinds are recognised:
//!   * **Fragment names** — when the cursor sits on the name token of a
//!     `th:fragment` definition, or on the referenced name token of a
//!     `th:insert`/`th:replace`/`th:include`, every fragment name token that
//!     matches is highlighted: definitions as [`DocumentHighlightKind::WRITE`],
//!     references as [`DocumentHighlightKind::READ`].
//!   * **Attribute names** — otherwise, if the cursor is on an attribute name,
//!     every occurrence of that same attribute name is highlighted as
//!     [`DocumentHighlightKind::TEXT`].
//!
//! Fragment name recognition and name-token ranges come from the shared
//! [`crate::fragmentref`] module, so the precise name token *inside* the
//! attribute value can be highlighted.

use std::ops::Range as ByteRange;

use tower_lsp::lsp_types::*;

use crate::document::{AttrOccurrence, Document};
use crate::fragmentref;

/// Computes the document highlights for the symbol under `position`.
///
/// Returns an empty vector when the cursor is not on a fragment name token or
/// an attribute name.
pub fn document_highlight(doc: &Document, position: Position) -> Vec<DocumentHighlight> {
    let offset = doc.offset_at(position);
    let attrs = doc.attributes();

    // 1. Fragment name token under the cursor -> highlight every def/ref.
    if let Some(name) = fragment_name_under_cursor(&attrs, offset) {
        if !name.is_empty() {
            return fragment_highlights(doc, &attrs, &name);
        }
    }

    // 2. Attribute name under the cursor -> highlight every same-named attr.
    if let Some(attr) = attrs
        .iter()
        .find(|a| offset >= a.name_range.start && offset <= a.name_range.end)
    {
        let name = attr.name.clone();
        return attrs
            .iter()
            .filter(|a| a.name == name)
            .map(|a| DocumentHighlight {
                range: to_range(doc, &a.name_range),
                kind: Some(DocumentHighlightKind::TEXT),
            })
            .collect();
    }

    Vec::new()
}

/// If `offset` falls inside the fragment *name token* of some attribute value,
/// returns that fragment name.
fn fragment_name_under_cursor(attrs: &[AttrOccurrence], offset: usize) -> Option<String> {
    for a in attrs {
        let base = a.value_range.start;
        let token = if fragmentref::is_fragment_attr(&a.name) {
            fragmentref::definition_name_range(&a.value)
        } else if fragmentref::is_reference_attr(&a.name) {
            fragmentref::reference_name_range(&a.value)
        } else {
            None
        };

        if let Some(r) = token {
            let start = base + r.start;
            let end = base + r.end;
            if offset >= start && offset <= end {
                return Some(a.value[r].to_string());
            }
        }
    }
    None
}

/// Highlights every fragment name token matching `name` across the document:
/// definitions as WRITE, references as READ.
fn fragment_highlights(
    doc: &Document,
    attrs: &[AttrOccurrence],
    name: &str,
) -> Vec<DocumentHighlight> {
    let mut out = Vec::new();
    for a in attrs {
        let base = a.value_range.start;
        if fragmentref::is_fragment_attr(&a.name) {
            if let Some(r) = fragmentref::definition_name_range(&a.value) {
                if &a.value[r.clone()] == name {
                    out.push(DocumentHighlight {
                        range: to_range(doc, &((base + r.start)..(base + r.end))),
                        kind: Some(DocumentHighlightKind::WRITE),
                    });
                }
            }
        } else if fragmentref::is_reference_attr(&a.name) {
            if let Some(r) = fragmentref::reference_name_range(&a.value) {
                if &a.value[r.clone()] == name {
                    out.push(DocumentHighlight {
                        range: to_range(doc, &((base + r.start)..(base + r.end))),
                        kind: Some(DocumentHighlightKind::READ),
                    });
                }
            }
        }
    }
    out
}

/// Converts a byte range into an LSP [`Range`].
fn to_range(doc: &Document, r: &ByteRange<usize>) -> Range {
    Range {
        start: doc.position_at(r.start),
        end: doc.position_at(r.end),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    fn doc(s: &str) -> Document {
        Document::new(s.to_string())
    }

    #[test]
    fn fragment_name_highlights_definition_and_reference() {
        let src = "<div th:fragment=\"header\">h</div>\n\
                   <div th:replace=\"~{self :: header}\"></div>";
        let d = doc(src);

        // Cursor on the definition's `header` name token.
        let idx = src.find("header").unwrap();
        let pos = d.position_at(idx + 2);

        let hls = document_highlight(&d, pos);
        assert_eq!(hls.len(), 2, "expected one def + one ref, got {:?}", hls);

        let writes = hls
            .iter()
            .filter(|h| h.kind == Some(DocumentHighlightKind::WRITE))
            .count();
        let reads = hls
            .iter()
            .filter(|h| h.kind == Some(DocumentHighlightKind::READ))
            .count();
        assert_eq!(writes, 1, "expected one WRITE (definition)");
        assert_eq!(reads, 1, "expected one READ (reference)");
    }

    #[test]
    fn fragment_reference_cursor_also_highlights_both() {
        let src = "<div th:fragment=\"header\">h</div>\n\
                   <div th:replace=\"~{self :: header}\"></div>";
        let d = doc(src);

        // Cursor on the `header` token inside the reference value.
        let ref_idx = src.rfind("header").unwrap();
        let pos = d.position_at(ref_idx + 2);

        let hls = document_highlight(&d, pos);
        assert_eq!(hls.len(), 2, "expected def + ref from a reference cursor");
    }

    #[test]
    fn attribute_name_highlights_repeated_occurrences() {
        let src = "<p th:text=\"a\">x</p>\n<span th:text=\"b\">y</span>";
        let d = doc(src);

        // Cursor on the first `th:text` attribute name.
        let idx = src.find("th:text").unwrap();
        let pos = d.position_at(idx + 2);

        let hls = document_highlight(&d, pos);
        assert_eq!(hls.len(), 2, "expected both th:text names highlighted");
        assert!(hls
            .iter()
            .all(|h| h.kind == Some(DocumentHighlightKind::TEXT)));
    }

    #[test]
    fn cursor_elsewhere_returns_empty() {
        let src = "<p th:text=\"a\">zz</p>";
        let d = doc(src);
        let idx = src.find("zz").unwrap();
        let pos = d.position_at(idx);
        assert!(document_highlight(&d, pos).is_empty());
    }
}
