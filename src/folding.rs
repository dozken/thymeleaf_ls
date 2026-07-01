//! Folding-range computation over the tree-sitter HTML tree.
//!
//! Editors request folding ranges to let users collapse structural blocks. We
//! surface a fold for every multi-line HTML `element`, `script_element`,
//! `style_element`, and `comment`. Everything else is skipped: single-line
//! nodes offer nothing to collapse.
//!
//! Fold spans run from the node's first line down to the line *before* its last
//! line, so the closing tag (e.g. `</div>`) stays visible when the block is
//! collapsed. When start and end would otherwise coincide the fold ends on the
//! last line instead, preserving a valid (if degenerate) range.

use tower_lsp::lsp_types::*;

use crate::document::Document;

/// Node kinds that we consider foldable, aside from comments (handled
/// separately so they can carry [`FoldingRangeKind::Comment`]).
const REGION_KINDS: &[&str] = &["element", "script_element", "style_element"];

/// Computes the folding ranges for `doc` by walking the tree-sitter tree.
///
/// Every foldable node whose start and end lie on different lines yields one
/// [`FoldingRange`]. Comments are tagged [`FoldingRangeKind::Comment`]; all
/// other foldable nodes are tagged [`FoldingRangeKind::Region`]. Identical
/// ranges are deduplicated while preserving source order.
pub fn folding_ranges(doc: &Document) -> Vec<FoldingRange> {
    let mut out: Vec<FoldingRange> = Vec::new();

    // Iterative pre-order traversal using index-based child access, mirroring
    // `Document::attributes` so nodes carry the tree lifetime.
    let mut stack = vec![doc.tree.root_node()];
    while let Some(node) = stack.pop() {
        let kind = node.kind();
        let is_comment = kind == "comment";
        let is_region = REGION_KINDS.contains(&kind);

        if is_comment || is_region {
            // Prefer tree-sitter's own line numbers (Point.row is 0-based, which
            // matches LSP). `position_at` is available as an alternative but the
            // Point already gives us exactly what we need.
            let start_line = node.start_position().row as u32;
            let end_line = node.end_position().row as u32;

            if end_line > start_line {
                // End one line early so the closing tag stays visible, unless
                // that would collapse the range past its start.
                let fold_end = end_line.saturating_sub(1);
                let fold_end = if fold_end > start_line {
                    fold_end
                } else {
                    end_line
                };

                let range = FoldingRange {
                    start_line,
                    start_character: None,
                    end_line: fold_end,
                    end_character: None,
                    kind: Some(if is_comment {
                        FoldingRangeKind::Comment
                    } else {
                        FoldingRangeKind::Region
                    }),
                    collapsed_text: None,
                };

                if !out.iter().any(|r| ranges_equal(r, &range)) {
                    out.push(range);
                }
            }
        }

        // Push children in reverse so they are visited in source order.
        for i in (0..node.child_count()).rev() {
            if let Some(child) = node.child(i) {
                stack.push(child);
            }
        }
    }

    out
}

/// Structural equality for two folding ranges (used for deduplication).
/// `FoldingRange` does not implement `PartialEq`, so we compare fields.
fn ranges_equal(a: &FoldingRange, b: &FoldingRange) -> bool {
    a.start_line == b.start_line
        && a.start_character == b.start_character
        && a.end_line == b.end_line
        && a.end_character == b.end_character
        && a.kind == b.kind
        && a.collapsed_text == b.collapsed_text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    fn doc(s: &str) -> Document {
        Document::new(s.to_string())
    }

    #[test]
    fn multiline_element_produces_a_fold() {
        let src = "<div>\n  <span>hi</span>\n</div>";
        let d = doc(src);
        let ranges = folding_ranges(&d);

        // The outer <div> spans lines 0..2; we expect a region fold covering it.
        let div_fold = ranges
            .iter()
            .find(|r| r.start_line == 0)
            .expect("expected a fold starting at the <div>");
        assert_eq!(div_fold.kind, Some(FoldingRangeKind::Region));
        assert!(
            div_fold.end_line > div_fold.start_line,
            "fold should span multiple lines, got {:?}",
            div_fold
        );
        // Closing tag on line 2 stays visible: fold ends before it.
        assert_eq!(div_fold.end_line, 1);
    }

    #[test]
    fn single_line_element_produces_no_fold() {
        let src = "<div><span>hi</span></div>";
        let d = doc(src);
        let ranges = folding_ranges(&d);
        assert!(
            ranges.is_empty(),
            "single-line elements should not fold, got {:?}",
            ranges
        );
    }

    #[test]
    fn multiline_comment_is_tagged_comment() {
        let src = "<!--\n  a comment\n-->";
        let d = doc(src);
        let ranges = folding_ranges(&d);
        let comment_fold = ranges
            .iter()
            .find(|r| r.kind == Some(FoldingRangeKind::Comment))
            .expect("expected a comment fold");
        assert!(comment_fold.end_line > comment_fold.start_line);
    }

    #[test]
    fn ranges_are_deduplicated() {
        let src = "<div>\n  <span>hi</span>\n</div>";
        let d = doc(src);
        let ranges = folding_ranges(&d);
        // No two ranges should be identical.
        for i in 0..ranges.len() {
            for j in (i + 1)..ranges.len() {
                assert!(
                    !ranges_equal(&ranges[i], &ranges[j]),
                    "found duplicate ranges: {:?}",
                    ranges[i]
                );
            }
        }
    }
}
