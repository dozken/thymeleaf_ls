//! Parsed-document abstraction: text + tree-sitter HTML tree, plus position
//! and cursor-context helpers used by the feature modules.

use std::ops::Range;

use tower_lsp::lsp_types::Position;
use tree_sitter::{Node, Parser, Tree};

/// A parsed HTML document: its full source text and the tree-sitter parse tree.
pub struct Document {
    pub text: String,
    pub tree: Tree,
}

/// Builds a fresh tree-sitter parser configured for the HTML grammar.
fn new_parser() -> Parser {
    let mut parser = Parser::new();
    parser
        .set_language(tree_sitter_html::language())
        .expect("failed to load tree-sitter-html grammar");
    parser
}

impl Document {
    /// Parses `text` into a new [`Document`].
    pub fn new(text: String) -> Document {
        let mut parser = new_parser();
        let tree = parser
            .parse(&text, None)
            .expect("tree-sitter html parse returned None");
        Document { text, tree }
    }

    /// Replaces the document text and re-parses it.
    ///
    /// This performs a full re-parse (no incremental editing) which matches the
    /// server's `TextDocumentSyncKind::FULL` sync mode.
    pub fn update(&mut self, text: String) {
        let mut parser = new_parser();
        let tree = parser
            .parse(&text, None)
            .expect("tree-sitter html parse returned None");
        self.text = text;
        self.tree = tree;
    }

    /// Applies a single incremental content change and re-parses.
    ///
    /// When `range` is `None` the whole document is replaced (a client may send
    /// a full-content change even under incremental sync). Otherwise the byte
    /// span covered by `range` is spliced out and replaced with `new_text`.
    pub fn apply_change(&mut self, range: Option<Position>, end: Option<Position>, new_text: &str) {
        match (range, end) {
            (Some(start), Some(end)) => {
                let start = self.offset_at(start);
                let end = self.offset_at(end);
                let (lo, hi) = if start <= end { (start, end) } else { (end, start) };
                self.text.replace_range(lo..hi, new_text);
            }
            _ => {
                self.text = new_text.to_string();
            }
        }
        let mut parser = new_parser();
        let tree = parser
            .parse(&self.text, None)
            .expect("tree-sitter html parse returned None");
        self.tree = tree;
    }

    // === Position <-> byte-offset conversions =============================
    //
    // POSITION ENCODING:
    // The LSP spec defaults to UTF-16 for the `character` component of a
    // `Position`. The server does not negotiate an alternative
    // `positionEncoding`, so `character` is interpreted as a count of UTF-16
    // code units within the line. Internally, byte offsets are used everywhere
    // else because that is what tree-sitter consumes, so these two helpers are
    // the single conversion boundary between the LSP (UTF-16) and tree-sitter
    // (UTF-8 byte) coordinate systems.

    /// Converts an LSP [`Position`] (line, UTF-16 `character`) to a byte offset
    /// into [`Document::text`]. Clamps to the end of the document / line, and
    /// always returns a value on a UTF-8 character boundary.
    pub fn offset_at(&self, pos: Position) -> usize {
        let bytes = self.text.as_bytes();

        // Advance to the start of the target line by counting newlines.
        let mut offset = 0usize;
        let mut line = 0u32;
        while line < pos.line && offset < bytes.len() {
            if bytes[offset] == b'\n' {
                line += 1;
            }
            offset += 1;
        }

        // Advance `character` UTF-16 code units into the line, stopping at the
        // newline / EOF. `offset` stays on a char boundary because it is only
        // ever advanced by whole UTF-8 characters.
        let mut utf16 = 0u32;
        while utf16 < pos.character && offset < bytes.len() && bytes[offset] != b'\n' {
            // `offset` is on a char boundary, so `chars().next()` yields the
            // character starting there.
            let Some(ch) = self.text[offset..].chars().next() else {
                break;
            };
            let next_utf16 = utf16 + ch.len_utf16() as u32;
            if next_utf16 > pos.character {
                // `character` falls inside a surrogate pair; clamp to the start
                // of this character rather than splitting it.
                break;
            }
            offset += ch.len_utf8();
            utf16 = next_utf16;
        }

        offset
    }

    /// Converts a byte offset into an LSP [`Position`] (line, UTF-16
    /// `character`). Clamps `offset` to the document length and to the nearest
    /// preceding UTF-8 character boundary.
    pub fn position_at(&self, offset: usize) -> Position {
        let mut offset = offset.min(self.text.len());
        while offset > 0 && !self.text.is_char_boundary(offset) {
            offset -= 1;
        }

        let bytes = self.text.as_bytes();
        let mut line = 0u32;
        let mut line_start = 0usize;
        let mut i = 0usize;
        while i < offset {
            if bytes[i] == b'\n' {
                line += 1;
                line_start = i + 1;
            }
            i += 1;
        }

        // `character` is the number of UTF-16 code units from the line start.
        let character: u32 = self.text[line_start..offset]
            .chars()
            .map(|c| c.len_utf16() as u32)
            .sum();

        Position { line, character }
    }

    // === Tree navigation ==================================================

    /// Returns the smallest *named* node whose byte range contains `offset`.
    #[allow(dead_code)]
    pub fn node_at(&self, offset: usize) -> Option<Node<'_>> {
        self.tree
            .root_node()
            .named_descendant_for_byte_range(offset, offset)
    }

    /// Determines the cursor context at `offset` (see [`CursorContext`]).
    pub fn context_at(&self, offset: usize) -> CursorContext {
        let root = self.tree.root_node();
        // Deepest node (named or not) touching the offset.
        let Some(node) = root.descendant_for_byte_range(offset, offset) else {
            return CursorContext::Other;
        };

        // Walk upward looking for an enclosing attribute, tag, or text node.
        let mut cur = Some(node);
        while let Some(n) = cur {
            match n.kind() {
                "attribute" => {
                    return self.attribute_context(n, offset);
                }
                "tag_name" => {
                    return CursorContext::TagName {
                        partial: self.node_text(n).to_string(),
                    };
                }
                "start_tag" | "self_closing_tag" => {
                    // Inside a tag but not on a specific attribute or the tag
                    // name (e.g. in the whitespace between attributes): treat
                    // as an attribute-name completion site.
                    let tag = self.tag_name_of(n).unwrap_or_default();
                    return CursorContext::AttrName {
                        tag,
                        partial: String::new(),
                    };
                }
                "text" => return CursorContext::Text,
                _ => {}
            }
            cur = n.parent();
        }

        CursorContext::Other
    }

    /// Builds the context for a cursor sitting somewhere within an `attribute`
    /// node, disambiguating between the name and the value.
    fn attribute_context(&self, attr: Node<'_>, offset: usize) -> CursorContext {
        let info = self.attr_info(attr);
        let tag = attr
            .parent()
            .and_then(|p| self.tag_name_of(p))
            .unwrap_or_default();

        // If the cursor is within the (quoted) value span, it's a value edit.
        if let Some(vspan) = &info.value_span {
            if offset >= vspan.start && offset <= vspan.end {
                return CursorContext::AttrValue {
                    tag,
                    attr: info.name.clone(),
                    value: info.value.clone(),
                };
            }
        }

        // Otherwise treat it as an attribute-name edit. `partial` is the text
        // of the name up to the cursor when the cursor is inside the name.
        let partial = if offset >= info.name_range.start && offset <= info.name_range.end {
            self.text[info.name_range.start..offset].to_string()
        } else {
            info.name.clone()
        };
        CursorContext::AttrName { tag, partial }
    }

    /// All attributes across the document, with names, values and byte ranges.
    pub fn attributes(&self) -> Vec<AttrOccurrence> {
        let mut out = Vec::new();
        // Iterative pre-order traversal over the whole tree, using index-based
        // child access so returned nodes carry the tree lifetime (rather than
        // borrowing a short-lived cursor).
        let mut stack = vec![self.tree.root_node()];
        while let Some(node) = stack.pop() {
            if node.kind() == "attribute" {
                let info = self.attr_info(node);
                let value_range = info
                    .value_range
                    .clone()
                    .unwrap_or(info.name_range.end..info.name_range.end);
                out.push(AttrOccurrence {
                    name: info.name,
                    value: info.value,
                    name_range: info.name_range,
                    value_range,
                });
            }
            for i in 0..node.child_count() {
                if let Some(child) = node.child(i) {
                    stack.push(child);
                }
            }
        }
        // Sort by position for a stable, source-order result.
        out.sort_by_key(|a| a.name_range.start);
        out
    }

    // === internals ========================================================

    /// Returns the source text spanned by `node`.
    fn node_text(&self, node: Node<'_>) -> &str {
        &self.text[node.byte_range()]
    }

    /// Finds the `tag_name` text of a `start_tag` / `self_closing_tag` /
    /// `element` node.
    fn tag_name_of(&self, node: Node<'_>) -> Option<String> {
        // For an element, descend into its start tag first.
        let container = if node.kind() == "element" {
            child_where(node, |ch| {
                matches!(ch.kind(), "start_tag" | "self_closing_tag")
            })
            .unwrap_or(node)
        } else {
            node
        };
        child_where(container, |ch| ch.kind() == "tag_name")
            .map(|ch| self.node_text(ch).to_string())
    }

    /// Extracts name / value strings and byte ranges from an `attribute` node.
    fn attr_info(&self, attr: Node<'_>) -> AttrInfo {
        let mut name = String::new();
        let mut name_range = attr.byte_range();
        let mut value = String::new();
        let mut value_range: Option<Range<usize>> = None;
        let mut value_span: Option<Range<usize>> = None;

        for i in 0..attr.child_count() {
            let Some(child) = attr.child(i) else { continue };
            match child.kind() {
                "attribute_name" => {
                    name = self.node_text(child).to_string();
                    name_range = child.byte_range();
                }
                // Unquoted value, e.g. `disabled=x`.
                "attribute_value" => {
                    value = self.node_text(child).to_string();
                    value_range = Some(child.byte_range());
                    value_span = Some(child.byte_range());
                }
                // Quoted value: the outer node includes the quotes; the inner
                // `attribute_value` is the content (absent for empty "").
                "quoted_attribute_value" => {
                    value_span = Some(child.byte_range());
                    if let Some(inner) = child_where(child, |g| g.kind() == "attribute_value") {
                        value = self.node_text(inner).to_string();
                        value_range = Some(inner.byte_range());
                    } else {
                        // Empty quotes: value range is just inside the quotes.
                        let r = child.byte_range();
                        let inner = (r.start + 1).min(r.end)..(r.end.saturating_sub(1)).max(r.start);
                        value_range = Some(inner);
                    }
                }
                _ => {}
            }
        }

        AttrInfo {
            name,
            name_range,
            value,
            value_range,
            value_span,
        }
    }
}

/// Returns the first direct child of `node` matching `pred`, using index-based
/// access so the returned node carries the tree lifetime (rather than borrowing
/// a short-lived `TreeCursor`).
fn child_where<'a>(node: Node<'a>, pred: impl Fn(&Node<'a>) -> bool) -> Option<Node<'a>> {
    (0..node.child_count())
        .filter_map(|i| node.child(i))
        .find(|ch| pred(ch))
}

/// A parsed attribute with its name, value, and byte ranges within the source.
#[derive(Debug, Clone)]
pub struct AttrOccurrence {
    pub name: String,
    pub value: String,
    /// Byte range of the attribute name.
    pub name_range: Range<usize>,
    /// Byte range of the attribute value (inner content, excluding quotes).
    pub value_range: Range<usize>,
}

/// Internal, richer attribute description used by context detection.
struct AttrInfo {
    name: String,
    name_range: Range<usize>,
    value: String,
    /// Inner value content range (excludes quotes).
    value_range: Option<Range<usize>>,
    /// Full value span *including* surrounding quotes, for hit-testing.
    value_span: Option<Range<usize>>,
}

/// What the cursor is sitting on, as far as the features need to know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CursorContext {
    /// Editing an attribute name inside a tag.
    AttrName { tag: String, partial: String },
    /// Editing an attribute value inside a tag.
    AttrValue {
        tag: String,
        attr: String,
        value: String,
    },
    /// Editing a tag name.
    TagName { partial: String },
    /// Inside element text content.
    Text,
    /// Anywhere else (comments, doctype, top-level whitespace, ...).
    Other,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(s: &str) -> Document {
        Document::new(s.to_string())
    }

    /// Byte offset of the middle of the first occurrence of `needle`.
    fn mid(text: &str, needle: &str) -> usize {
        let start = text.find(needle).expect("needle present");
        start + needle.len() / 2
    }

    #[test]
    fn context_at_attr_name() {
        let src = "<p th:text=\"hi\">Z</p>";
        let d = doc(src);
        match d.context_at(mid(src, "th:text")) {
            CursorContext::AttrName { tag, .. } => assert_eq!(tag, "p"),
            other => panic!("expected AttrName, got {:?}", other),
        }
    }

    #[test]
    fn context_at_attr_value() {
        let src = "<p th:text=\"hi\">Z</p>";
        let d = doc(src);
        match d.context_at(mid(src, "hi")) {
            CursorContext::AttrValue { tag, attr, value } => {
                assert_eq!(tag, "p");
                assert_eq!(attr, "th:text");
                assert_eq!(value, "hi");
            }
            other => panic!("expected AttrValue, got {:?}", other),
        }
    }

    #[test]
    fn context_at_text() {
        let src = "<p th:text=\"hi\">Z</p>";
        let d = doc(src);
        let off = src.find('Z').unwrap();
        assert_eq!(d.context_at(off), CursorContext::Text);
    }

    #[test]
    fn context_at_empty_tag_is_attr_name_site() {
        // Whitespace between the tag name and `>` is an attribute-name site.
        let src = "<div  ></div>";
        let d = doc(src);
        let off = src.find("  ").unwrap() + 1;
        match d.context_at(off) {
            CursorContext::AttrName { tag, partial } => {
                assert_eq!(tag, "div");
                assert!(partial.is_empty());
            }
            other => panic!("expected AttrName, got {:?}", other),
        }
    }

    #[test]
    fn offset_position_roundtrip() {
        let src = "<p>\n  <span>hi</span>\n</p>";
        let d = doc(src);
        let pos = Position { line: 1, character: 3 };
        let off = d.offset_at(pos);
        assert_eq!(d.position_at(off), pos);
    }

    #[test]
    fn offset_at_uses_utf16_code_units() {
        // "café" is 4 chars / 5 bytes / 4 UTF-16 units. The attribute name that
        // follows should be addressed via UTF-16 code units per the LSP default.
        let src = "<p title=\"café\" th:text=\"x\">y</p>";
        let d = doc(src);
        // UTF-16 character index of the 't' in "th:text".
        let byte_idx = src.find("th:text").unwrap();
        let utf16_char = src[..byte_idx].chars().map(|c| c.len_utf16()).sum::<usize>() as u32;
        let off = d.offset_at(Position { line: 0, character: utf16_char });
        assert_eq!(off, byte_idx, "offset_at should map UTF-16 units to byte offset");
        // And roundtrip back.
        assert_eq!(
            d.position_at(byte_idx),
            Position { line: 0, character: utf16_char }
        );
    }

    #[test]
    fn attributes_collects_name_and_value() {
        let src = "<p th:text=\"${a}\">x</p>";
        let d = doc(src);
        let attrs = d.attributes();
        assert_eq!(attrs.len(), 1);
        assert_eq!(attrs[0].name, "th:text");
        assert_eq!(attrs[0].value, "${a}");
    }
}
