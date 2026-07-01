//! Quick-fix code actions for Thymeleaf attributes.
//!
//! Operates purely on a parsed [`Document`]. For each `th:*` / `data-th-*`
//! attribute whose name overlaps the requested range and is *not* part of the
//! known Standard Dialect catalog, this module offers:
//!  * a "Change to `th:xxx`" quick fix that rewrites the name to the closest
//!    known attribute (by Levenshtein distance, only when distance <= 3), and
//!  * a "Remove `name`" quick fix that deletes the whole attribute.

use std::collections::HashMap;

use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Range, TextEdit, Url, WorkspaceEdit,
};

use crate::{document::Document, thymeleaf};

/// Maximum Levenshtein distance at which a "change to" suggestion is offered.
const MAX_SUGGESTION_DISTANCE: usize = 3;

/// Computes the code actions available for `range` in `doc`.
///
/// Returns quick fixes for the unknown Thymeleaf attributes whose name range
/// overlaps the requested `range`.
pub fn code_actions(doc: &Document, uri: &Url, range: Range) -> Vec<CodeActionOrCommand> {
    let mut out = Vec::new();

    // Byte offsets of the requested range, for overlap testing.
    let req_start = doc.offset_at(range.start);
    let req_end = doc.offset_at(range.end);

    for attr in doc.attributes() {
        if !is_thymeleaf_name(&attr.name) {
            continue;
        }
        // Only unknown attributes are actionable.
        if thymeleaf::lookup(&attr.name).is_some() {
            continue;
        }
        // The attribute's name range must overlap the requested range. Two
        // ranges [a,b) and [c,d) overlap iff a < d && c < b; also allow the
        // common zero-width cursor case where start == end.
        let (n_start, n_end) = (attr.name_range.start, attr.name_range.end);
        let overlaps = n_start <= req_end && req_start <= n_end;
        if !overlaps {
            continue;
        }

        // a) "Change to" the closest known attribute name.
        if let Some(suggestion) = closest_known(&attr.name) {
            let name_lsp = Range {
                start: doc.position_at(n_start),
                end: doc.position_at(n_end),
            };
            let edit = workspace_edit(uri, name_lsp, suggestion.to_string());
            out.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: format!("Change to `{}`", suggestion),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: None,
                edit: Some(edit),
                is_preferred: Some(true),
                ..Default::default()
            }));
        }

        // b) "Remove attribute": delete name through end of value including the
        // closing quote. Approximate the span and clamp to document bounds.
        let del_end = (attr.value_range.end + 1).min(doc.text.len()).max(n_end);
        let remove_lsp = Range {
            start: doc.position_at(n_start),
            end: doc.position_at(del_end),
        };
        let edit = workspace_edit(uri, remove_lsp, String::new());
        out.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: format!("Remove `{}`", attr.name),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: None,
            edit: Some(edit),
            is_preferred: None,
            ..Default::default()
        }));
    }

    out
}

/// Whether `name` looks like a Thymeleaf attribute (`th:*` or `data-th-*`),
/// case-insensitively.
fn is_thymeleaf_name(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    lower.starts_with("th:") || lower.starts_with("data-th-")
}

/// Returns the canonical name of the closest known attribute to `name`, if it
/// is within [`MAX_SUGGESTION_DISTANCE`] edits.
fn closest_known(name: &str) -> Option<&'static str> {
    let target = canonical(name);
    thymeleaf::all_attrs()
        .iter()
        .map(|a| (a.name, levenshtein(&target, a.name)))
        .filter(|(_, dist)| *dist <= MAX_SUGGESTION_DISTANCE)
        .min_by_key(|(_, dist)| *dist)
        .map(|(candidate, _)| candidate)
}

/// Normalizes an attribute name to canonical `th:xxx` form for comparison.
/// Falls back to the lowercased verbatim name if it is not in a recognizable
/// Thymeleaf form.
fn canonical(name: &str) -> String {
    let lower = name.trim().to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("data-th-") {
        format!("th:{}", rest)
    } else if let Some(rest) = lower.strip_prefix("th:") {
        format!("th:{}", rest)
    } else {
        lower
    }
}

/// Builds a single-edit [`WorkspaceEdit`] replacing `range` in `uri` with
/// `new_text`.
fn workspace_edit(uri: &Url, range: Range, new_text: String) -> WorkspaceEdit {
    let mut changes = HashMap::new();
    changes.insert(uri.clone(), vec![TextEdit { range, new_text }]);
    WorkspaceEdit {
        changes: Some(changes),
        document_changes: None,
        change_annotations: None,
    }
}

/// Classic dynamic-programming Levenshtein (edit) distance between two strings,
/// counting insertions, deletions and substitutions. Operates over Unicode
/// scalar values.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }

    // Single-row rolling buffer of previous distances.
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];

    for (i, &ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            cur[j + 1] = (prev[j + 1] + 1) // deletion
                .min(cur[j] + 1) // insertion
                .min(prev[j] + cost); // substitution
        }
        std::mem::swap(&mut prev, &mut cur);
    }

    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    fn doc(s: &str) -> Document {
        Document::new(s.to_string())
    }

    fn uri() -> Url {
        Url::parse("file:///test.html").unwrap()
    }

    /// A range covering the whole document, so every attribute overlaps.
    fn whole(d: &Document) -> Range {
        Range {
            start: d.position_at(0),
            end: d.position_at(d.text.len()),
        }
    }

    fn titles(actions: &[CodeActionOrCommand]) -> Vec<String> {
        actions
            .iter()
            .map(|a| match a {
                CodeActionOrCommand::CodeAction(ca) => ca.title.clone(),
                CodeActionOrCommand::Command(c) => c.title.clone(),
            })
            .collect()
    }

    #[test]
    fn unknown_attr_yields_suggestion_and_remove() {
        let d = doc("<p th:txt=\"hi\">Z</p>");
        let actions = code_actions(&d, &uri(), whole(&d));
        let ts = titles(&actions);
        assert!(
            ts.iter().any(|t| t == "Change to `th:text`"),
            "expected a suggestion to th:text, got {:?}",
            ts
        );
        assert!(
            ts.iter().any(|t| t == "Remove `th:txt`"),
            "expected a remove action, got {:?}",
            ts
        );
    }

    #[test]
    fn suggestion_is_preferred_and_quickfix() {
        let d = doc("<p th:txt=\"hi\">Z</p>");
        let actions = code_actions(&d, &uri(), whole(&d));
        let change = actions
            .iter()
            .find_map(|a| match a {
                CodeActionOrCommand::CodeAction(ca) if ca.title == "Change to `th:text`" => Some(ca),
                _ => None,
            })
            .expect("change action present");
        assert_eq!(change.kind, Some(CodeActionKind::QUICKFIX));
        assert_eq!(change.is_preferred, Some(true));
        assert!(change.edit.is_some());
    }

    #[test]
    fn known_attr_yields_no_actions() {
        let d = doc("<p th:text=\"hi\">Z</p>");
        let actions = code_actions(&d, &uri(), whole(&d));
        assert!(actions.is_empty(), "unexpected actions: {:?}", titles(&actions));
    }

    #[test]
    fn far_off_name_has_no_suggestion_but_still_removable() {
        // Distance from "th:completelywrong" to any known name exceeds 3.
        let d = doc("<p th:completelywrong=\"x\">Z</p>");
        let actions = code_actions(&d, &uri(), whole(&d));
        let ts = titles(&actions);
        assert!(!ts.iter().any(|t| t.starts_with("Change to")));
        assert!(ts.iter().any(|t| t == "Remove `th:completelywrong`"));
    }

    #[test]
    fn range_outside_attribute_yields_nothing() {
        let d = doc("<p th:txt=\"hi\">Z</p>");
        // Point the range at the trailing "Z" text, away from the attribute.
        let z = d.text.find('Z').unwrap();
        let range = Range {
            start: d.position_at(z),
            end: d.position_at(z + 1),
        };
        let actions = code_actions(&d, &uri(), range);
        assert!(actions.is_empty(), "unexpected actions: {:?}", titles(&actions));
    }

    #[test]
    fn levenshtein_basic() {
        assert_eq!(levenshtein("th:txt", "th:text"), 1);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", "abc"), 0);
    }
}
