//! Hover feature: shows documentation for Thymeleaf attributes and, when the
//! cursor sits inside a `th:*` attribute value, contextual help for the
//! Thymeleaf expression syntax / utility object under the cursor.

use tower_lsp::lsp_types::*;

use crate::{
    document::{AttrOccurrence, Document},
    thymeleaf,
    vault::Vault,
};

/// Produces hover information for the symbol at `position` in the document
/// identified by `uri`.
///
/// Resolution order:
/// 1. If the cursor is on a known Thymeleaf attribute *name*, return the
///    attribute's documentation, anchored to the attribute-name range.
/// 2. If the cursor is inside a `th:*` attribute *value*, return help for the
///    expression syntax / utility object token under the cursor (if any),
///    anchored to that token's range.
/// 3. Otherwise return `None`.
pub fn hover(vault: &Vault, uri: &Url, position: Position) -> Option<Hover> {
    let doc = vault.get(uri)?;
    let offset = doc.offset_at(position);
    let attrs = doc.attributes();

    // 1) Cursor on an attribute name.
    if let Some(attr) = attrs
        .iter()
        .find(|a| offset >= a.name_range.start && offset <= a.name_range.end)
    {
        if let Some(meta) = thymeleaf::lookup(&attr.name) {
            let markdown = format!("## `{}`\n\n{}\n\n{}", meta.name, meta.summary, meta.doc);
            let range = Range {
                start: doc.position_at(attr.name_range.start),
                end: doc.position_at(attr.name_range.end),
            };
            return Some(hover_markdown(markdown, Some(range)));
        }
    }

    // 2) Cursor inside a th:* attribute value: contextual expression help.
    if let Some(attr) = attrs
        .iter()
        .find(|a| offset >= a.value_range.start && offset <= a.value_range.end)
    {
        if is_thymeleaf_attr(&attr.name) {
            if let Some((markdown, range)) = expression_help_at(doc, attr, offset) {
                return Some(hover_markdown(markdown.to_string(), Some(range)));
            }
        }
    }

    None
}

/// Whether `name` denotes any Thymeleaf attribute (`th:*` / `data-th-*`),
/// regardless of whether it is in the known catalog.
fn is_thymeleaf_attr(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    lower.starts_with("th:") || lower.starts_with("data-th-")
}

/// Builds a Markdown [`Hover`].
fn hover_markdown(value: String, range: Option<Range>) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range,
    }
}

/// Finds the expression-syntax / utility-object token under the cursor within an
/// attribute value and returns its documentation markdown together with the LSP
/// range of the token.
fn expression_help_at(
    doc: &Document,
    attr: &AttrOccurrence,
    offset: usize,
) -> Option<(&'static str, Range)> {
    // Offset within the (quote-stripped) value string.
    let value = &attr.value;
    let base = attr.value_range.start;
    let rel = offset.saturating_sub(base).min(value.len());

    // Prefer the more specific utility-object tokens (e.g. `#dates`) over the
    // generic brace expressions (`${...}`), since utilities usually appear
    // *inside* a `${...}` expression.
    let mut best: Option<(usize, usize, &'static str)> = None; // (span_start, span_len, md)

    for &(token, md) in thymeleaf::expression_syntaxes() {
        let is_brace = token.contains('{');
        if is_brace {
            // Opening literal is the first two characters, e.g. "${", "*{".
            let open: String = token.chars().take(2).collect();
            let mut from = 0usize;
            while let Some(idx) = value[from..].find(&open) {
                let start = from + idx;
                // Span extends to the matching '}' (inclusive) or end of value.
                let end = value[start..]
                    .find('}')
                    .map(|j| start + j + 1)
                    .unwrap_or(value.len());
                if rel >= start && rel <= end {
                    consider(&mut best, start, end - start, md, /* utility */ false);
                }
                from = start + open.len();
            }
        } else {
            // Literal utility-object token, e.g. "#dates".
            let mut from = 0usize;
            while let Some(idx) = value[from..].find(token) {
                let start = from + idx;
                let end = start + token.len();
                if rel >= start && rel <= end {
                    consider(&mut best, start, token.len(), md, /* utility */ true);
                }
                from = end;
            }
        }
    }

    let (start, len, md) = best?;
    let range = Range {
        start: doc.position_at(base + start),
        end: doc.position_at(base + start + len),
    };
    Some((md, range))
}

/// Keeps the best candidate token match. Utility-object matches win over
/// generic brace expressions; among same kind the shorter (tighter) span wins.
fn consider(
    best: &mut Option<(usize, usize, &'static str)>,
    start: usize,
    len: usize,
    md: &'static str,
    utility: bool,
) {
    let priority = |is_util: bool, span_len: usize| (if is_util { 0 } else { 1 }, span_len);
    match best {
        None => *best = Some((start, len, md)),
        Some((_, best_len, best_md)) => {
            let best_is_util = is_utility_md(best_md);
            if priority(utility, len) < priority(best_is_util, *best_len) {
                *best = Some((start, len, md));
            }
        }
    }
}

/// Heuristic to recover whether a stored markdown corresponds to a utility
/// object (its description does not start with the bold "**...expression**").
fn is_utility_md(md: &str) -> bool {
    !md.starts_with("**")
}
