//! Workspace-wide rename for Thymeleaf fragments.
//!
//! A fragment is *defined* by `th:fragment="name(args)"` and *referenced* by
//! `th:insert`/`th:replace`/`th:include`, whose values look like
//! `~{template :: name}`, `template :: name`, `:: name`, or a bare `name`.
//!
//! Renaming a fragment must touch only the *name token* wherever it appears:
//!   * in a definition, the `name` part is rewritten while the `(args)` suffix
//!     is preserved;
//!   * in a reference, the `name` part is rewritten while the `~{template :: }`
//!     wrapper (and any `(args)`) is preserved.
//!
//! Both [`prepare_rename`] and [`rename`] first locate the name token under the
//! cursor, then compute precise byte ranges *within* each attribute value so
//! that surrounding syntax is never disturbed.

use std::collections::HashMap;

use tower_lsp::lsp_types::*;

use crate::document::{AttrOccurrence, Document};
use crate::vault::Vault;

/// Rename-prepare: if the cursor sits on a fragment name token (inside a
/// `th:fragment` definition's name part, or inside a fragment-reference's name
/// part), return the exact [`Range`] covering just that name token. Otherwise
/// [`None`].
pub fn prepare_rename(vault: &Vault, uri: &Url, position: Position) -> Option<Range> {
    let doc = vault.get(uri)?;
    let offset = doc.offset_at(position);
    let (_name, span) = name_token_at(doc, offset)?;
    Some(Range {
        start: doc.position_at(span.start),
        end: doc.position_at(span.end),
    })
}

/// Rename: determine the fragment name under the cursor, then across every
/// document in the vault build [`TextEdit`]s that replace only the name token in
/// each `th:fragment` definition and each fragment reference. Returns a
/// [`WorkspaceEdit`], or [`None`] if there is no fragment under the cursor or no
/// occurrences to rewrite.
pub fn rename(
    vault: &Vault,
    uri: &Url,
    position: Position,
    new_name: &str,
) -> Option<WorkspaceEdit> {
    let doc = vault.get(uri)?;
    let offset = doc.offset_at(position);
    let (name, _span) = name_token_at(doc, offset)?;
    if name.is_empty() {
        return None;
    }

    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
    for edit_uri in vault.uris().cloned().collect::<Vec<_>>() {
        let Some(edit_doc) = vault.get(&edit_uri) else {
            continue;
        };
        let mut edits = Vec::new();
        for attr in edit_doc.attributes() {
            let Some((token_name, span)) = attr_name_token(&attr) else {
                continue;
            };
            if token_name != name {
                continue;
            }
            edits.push(TextEdit {
                range: Range {
                    start: edit_doc.position_at(span.start),
                    end: edit_doc.position_at(span.end),
                },
                new_text: new_name.to_string(),
            });
        }
        if !edits.is_empty() {
            changes.insert(edit_uri, edits);
        }
    }

    if changes.is_empty() {
        None
    } else {
        Some(WorkspaceEdit {
            changes: Some(changes),
            ..Default::default()
        })
    }
}

/// Finds the fragment name token under `offset`: returns the fragment name and
/// the *absolute* byte range (into the document) covering just that name token.
fn name_token_at(doc: &Document, offset: usize) -> Option<(String, std::ops::Range<usize>)> {
    for attr in doc.attributes() {
        let Some((name, span)) = attr_name_token(&attr) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        if offset >= span.start && offset <= span.end {
            return Some((name, span));
        }
    }
    None
}

/// For a single attribute, returns the fragment name and the absolute byte range
/// of its name token, if the attribute is a `th:fragment` definition or a
/// fragment reference (and a name is present).
fn attr_name_token(attr: &AttrOccurrence) -> Option<(String, std::ops::Range<usize>)> {
    let span = if is_fragment_attr(&attr.name) {
        definition_name_span(&attr.value)?
    } else if is_reference_attr(&attr.name) {
        reference_name_span(&attr.value)?
    } else {
        return None;
    };
    let name = attr.value[span.clone()].to_string();
    if name.is_empty() {
        return None;
    }
    let abs = (attr.value_range.start + span.start)..(attr.value_range.start + span.end);
    Some((name, abs))
}

// === Attribute-name classification ========================================
//
// Replicated from `navigation.rs` (its helpers are private) so behaviour stays
// identical without editing that module.

/// True if `name` denotes a `th:fragment` definition (accepts `data-th-`).
fn is_fragment_attr(name: &str) -> bool {
    matches_th_attr(name, "fragment")
}

/// True if `name` denotes a fragment-reference attribute: `th:insert`,
/// `th:replace`, or `th:include` (accepts the `data-th-` form).
fn is_reference_attr(name: &str) -> bool {
    matches_th_attr(name, "insert")
        || matches_th_attr(name, "replace")
        || matches_th_attr(name, "include")
}

/// Case-insensitively matches an attribute name against `th:<local>` or
/// `data-th-<local>`.
fn matches_th_attr(name: &str, local: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    lower == format!("th:{local}") || lower == format!("data-th-{local}")
}

// === Name-token span computation ==========================================
//
// These mirror `navigation.rs`'s `parse_definition_name` / `parse_reference_name`
// but return the byte range of the name *within the value string* so the exact
// token can be rewritten in place.

/// Byte range (within `value`) of the fragment name in a `th:fragment` value,
/// e.g. `"header(title)"` -> `0..6`. `None` if the name is empty.
fn definition_name_span(value: &str) -> Option<std::ops::Range<usize>> {
    let start = value.len() - value.trim_start().len();
    let after = &value[start..];
    let name_part = match after.find('(') {
        Some(idx) => &after[..idx],
        None => after,
    };
    let name = name_part.trim_end();
    if name.is_empty() {
        return None;
    }
    Some(start..start + name.len())
}

/// Byte range (within `value`) of the referenced fragment name in a
/// `th:insert`/`th:replace`/`th:include` value. Handles `~{template :: name}`,
/// `template :: name`, `:: name`, and bare `name`, stripping any `(args)`.
/// `None` if the name is empty.
fn reference_name_span(value: &str) -> Option<std::ops::Range<usize>> {
    // Maintain a `[lo, hi)` window into `value` mirroring the parse in
    // `navigation::parse_reference_name`, narrowing it step by step.
    let mut lo = value.len() - value.trim_start().len();
    let mut hi = value.trim_end().len();
    if lo >= hi {
        return None;
    }

    // Strip an outer `~{ ... }` wrapper.
    let s = &value[lo..hi];
    if let Some(rest) = s.strip_prefix("~{") {
        lo += 2;
        lo += rest.len() - rest.trim_start().len();
        if value[lo..hi].ends_with('}') {
            hi -= 1;
        }
    }
    trim_window(value, &mut lo, &mut hi);

    // The selector is the segment after the last `::`.
    if let Some(idx) = value[lo..hi].rfind("::") {
        lo = lo + idx + 2;
    }
    trim_window(value, &mut lo, &mut hi);

    // A trailing `}` may remain if the wrapper suffix wasn't cleanly stripped.
    if value[lo..hi].ends_with('}') {
        hi -= 1;
    }

    // Drop any argument list: `name(args)` -> `name`.
    if let Some(idx) = value[lo..hi].find('(') {
        hi = lo + idx;
    }
    trim_window(value, &mut lo, &mut hi);

    if lo >= hi {
        return None;
    }
    Some(lo..hi)
}

/// Shrinks `[lo, hi)` to skip leading/trailing ASCII/Unicode whitespace.
fn trim_window(value: &str, lo: &mut usize, hi: &mut usize) {
    let s = &value[*lo..*hi];
    *lo += s.len() - s.trim_start().len();
    let s = &value[*lo..*hi];
    *hi -= s.len() - s.trim_end().len();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vault_with(entries: &[(&str, &str)]) -> (Vault, Vec<Url>) {
        let mut vault = Vault::new(None);
        let mut uris = Vec::new();
        for (path, text) in entries {
            let uri = Url::parse(path).unwrap();
            vault.upsert(uri.clone(), text.to_string());
            uris.push(uri);
        }
        (vault, uris)
    }

    /// Byte offset of the middle of the first occurrence of `needle`.
    fn mid_pos(vault: &Vault, uri: &Url, text: &str, needle: &str) -> Position {
        let start = text.find(needle).expect("needle present");
        let off = start + needle.len() / 2;
        vault.get(uri).unwrap().position_at(off)
    }

    #[test]
    fn definition_name_span_drops_args() {
        assert_eq!(definition_name_span("header(title)"), Some(0..6));
        assert_eq!(definition_name_span("  footer "), Some(2..8));
        assert_eq!(definition_name_span("   "), None);
    }

    #[test]
    fn reference_name_span_wrapped() {
        let v = "~{tpl :: header}";
        assert_eq!(&v[reference_name_span(v).unwrap()], "header");
    }

    #[test]
    fn reference_name_span_various_forms() {
        let v = "template :: name";
        assert_eq!(&v[reference_name_span(v).unwrap()], "name");
        let v = ":: name";
        assert_eq!(&v[reference_name_span(v).unwrap()], "name");
        let v = "~{fragments :: header('Home')}";
        assert_eq!(&v[reference_name_span(v).unwrap()], "header");
        let v = "bare";
        assert_eq!(&v[reference_name_span(v).unwrap()], "bare");
    }

    #[test]
    fn prepare_rename_on_definition() {
        let text = "<div th:fragment=\"header(title)\">h</div>";
        let (vault, uris) = vault_with(&[("file:///a.html", text)]);
        let pos = mid_pos(&vault, &uris[0], text, "header");
        let range = prepare_rename(&vault, &uris[0], pos).expect("some on definition");
        // Range covers exactly "header".
        let doc = vault.get(&uris[0]).unwrap();
        let start = doc.offset_at(range.start);
        let end = doc.offset_at(range.end);
        assert_eq!(&text[start..end], "header");
    }

    #[test]
    fn prepare_rename_on_reference() {
        let text = "<div th:replace=\"~{tpl :: header}\"></div>";
        let (vault, uris) = vault_with(&[("file:///a.html", text)]);
        let pos = mid_pos(&vault, &uris[0], text, "header");
        let range = prepare_rename(&vault, &uris[0], pos).expect("some on reference");
        let doc = vault.get(&uris[0]).unwrap();
        let start = doc.offset_at(range.start);
        let end = doc.offset_at(range.end);
        assert_eq!(&text[start..end], "header");
    }

    #[test]
    fn prepare_rename_none_on_plain_text() {
        let text = "<div>hello header world</div>";
        let (vault, uris) = vault_with(&[("file:///a.html", text)]);
        let pos = mid_pos(&vault, &uris[0], text, "header");
        assert!(prepare_rename(&vault, &uris[0], pos).is_none());
    }

    #[test]
    fn rename_across_two_docs() {
        let def_text = "<div th:fragment=\"header(title)\">h</div>";
        let ref_text = "<div th:replace=\"~{tpl :: header}\"></div>";
        let (vault, uris) = vault_with(&[
            ("file:///frag.html", def_text),
            ("file:///page.html", ref_text),
        ]);
        let def_uri = &uris[0];
        let ref_uri = &uris[1];

        // Rename starting from the definition.
        let pos = mid_pos(&vault, def_uri, def_text, "header");
        let ws = rename(&vault, def_uri, pos, "banner").expect("some workspace edit");
        let changes = ws.changes.expect("changes present");

        // Edit in the defining document rewrites only "header".
        let def_edits = changes.get(def_uri).expect("edit in def doc");
        assert_eq!(def_edits.len(), 1);
        assert_eq!(def_edits[0].new_text, "banner");
        let dd = vault.get(def_uri).unwrap();
        let s = dd.offset_at(def_edits[0].range.start);
        let e = dd.offset_at(def_edits[0].range.end);
        assert_eq!(&def_text[s..e], "header");

        // Edit in the referencing document rewrites only "header".
        let ref_edits = changes.get(ref_uri).expect("edit in ref doc");
        assert_eq!(ref_edits.len(), 1);
        assert_eq!(ref_edits[0].new_text, "banner");
        let rd = vault.get(ref_uri).unwrap();
        let s = rd.offset_at(ref_edits[0].range.start);
        let e = rd.offset_at(ref_edits[0].range.end);
        assert_eq!(&ref_text[s..e], "header");
    }

    #[test]
    fn rename_none_when_no_fragment_under_cursor() {
        let text = "<div>plain</div>";
        let (vault, uris) = vault_with(&[("file:///a.html", text)]);
        let pos = mid_pos(&vault, &uris[0], text, "plain");
        assert!(rename(&vault, &uris[0], pos, "x").is_none());
    }
}
