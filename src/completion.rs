//! Completion feature: Thymeleaf attribute-name and expression-syntax
//! completions, driven by the cursor context reported by [`Document`].

use tower_lsp::lsp_types::*;

use crate::{
    document::CursorContext,
    thymeleaf,
    vault::Vault,
};

/// Produces completion items for the cursor at `position` in `uri`.
///
/// * In an attribute-name context (typing inside a start tag), offers all
///   Thymeleaf standard-dialect attributes filtered by the partial text.
/// * In the value of a `th:*` attribute, offers expression-syntax helpers and
///   utility objects.
/// * Everywhere else, returns an empty list.
pub fn completion(
    vault: &Vault,
    uri: &Url,
    position: Position,
) -> Vec<CompletionItem> {
    let Some(doc) = vault.get(uri) else {
        return Vec::new();
    };
    let offset = doc.offset_at(position);

    match doc.context_at(offset) {
        CursorContext::AttrName { partial, .. } => attribute_completions(&partial),
        CursorContext::AttrValue { attr, .. } => {
            // Only offer expression helpers inside recognized Thymeleaf
            // attributes.
            if thymeleaf::lookup(&attr).is_some() {
                expression_completions()
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    }
}

/// Builds completion items for all Thymeleaf attributes whose canonical name
/// matches `partial`.
fn attribute_completions(partial: &str) -> Vec<CompletionItem> {
    let filter = normalize_partial(partial);

    thymeleaf::all_attrs()
        .iter()
        .filter(|attr| filter.is_empty() || attr.name.starts_with(&filter))
        .map(|attr| CompletionItem {
            label: attr.name.to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some(attr.summary.to_string()),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: attr.doc.to_string(),
            })),
            insert_text: Some(attr.name.to_string()),
            ..Default::default()
        })
        .collect()
}

/// Normalizes the partial attribute text into a canonical `th:xxx`-style prefix
/// suitable for `starts_with` filtering against the catalog.
///
/// Handles the various states while typing:
/// * `""` / `"t"` / `"th"` -> `"th:"`-family prefix (all attrs offered)
/// * `"th:"` / `"th:te"` -> `"th:te"`
/// * `"data-th-te"` -> `"th:te"` (HTML5 form normalized)
fn normalize_partial(partial: &str) -> String {
    let lower = partial.trim().to_ascii_lowercase();

    // HTML5 `data-th-*` spelling.
    if let Some(rest) = lower.strip_prefix("data-th-") {
        return format!("th:{}", rest);
    }
    if let Some(rest) = lower.strip_prefix("data-th") {
        // `data-th` (no trailing dash yet) — offer everything.
        let _ = rest;
        return String::new();
    }

    // Canonical `th:*` spelling (also matches a bare trailing ':' trigger,
    // e.g. the user typed `th` and triggered on ':').
    if let Some(rest) = lower.strip_prefix("th:") {
        return format!("th:{}", rest);
    }
    if lower == "th" || lower == "t" || lower.is_empty() {
        // Prefix of `th:` — offer the full catalog.
        return String::new();
    }

    // Some other partial that is a prefix of "th:" (defensive) or unrelated.
    if "th:".starts_with(&lower) {
        return String::new();
    }

    // Unrelated text: return it verbatim so nothing matches.
    lower
}

/// Builds completion items for the Thymeleaf expression syntaxes and utility
/// objects.
fn expression_completions() -> Vec<CompletionItem> {
    thymeleaf::expression_syntaxes()
        .iter()
        .map(|(token, markdown)| {
            let is_expression = token.ends_with("{...}");
            let insert_text = if is_expression {
                // Insert the wrapper with the cursor placed inside the braces.
                let stripped = token.trim_end_matches("{...}");
                format!("{}{{}}", stripped)
            } else {
                // Utility object token, e.g. `#strings`.
                token.to_string()
            };

            CompletionItem {
                label: token.to_string(),
                kind: Some(if is_expression {
                    CompletionItemKind::SNIPPET
                } else {
                    CompletionItemKind::FUNCTION
                }),
                detail: Some(if is_expression {
                    "Thymeleaf expression".to_string()
                } else {
                    "Thymeleaf utility object".to_string()
                }),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: markdown.to_string(),
                })),
                insert_text: Some(insert_text),
                ..Default::default()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vault::Vault;

    /// Builds a single-document vault and returns it with the doc's URL.
    fn vault_with(src: &str) -> (Vault, Url) {
        let uri = Url::parse("file:///test.html").unwrap();
        let mut vault = Vault::new(None);
        vault.upsert(uri.clone(), src.to_string());
        (vault, uri)
    }

    /// Position at a byte offset on the (single) first line.
    fn pos(offset: usize) -> Position {
        Position {
            line: 0,
            character: offset as u32,
        }
    }

    #[test]
    fn attr_name_context_returns_thymeleaf_attrs() {
        let src = "<div  ></div>";
        let (vault, uri) = vault_with(src);
        let off = src.find("  ").unwrap() + 1;
        let items = completion(&vault, &uri, pos(off));
        assert!(!items.is_empty());
        assert!(items.iter().any(|i| i.label == "th:text"));
        assert!(items.iter().any(|i| i.label == "th:if"));
        // Every offered item is a Thymeleaf attribute.
        assert!(items.iter().all(|i| i.label.starts_with("th:")));
    }

    #[test]
    fn attr_name_context_filters_by_partial() {
        // Cursor inside a partial `th:t...` attribute name.
        let src = "<div th:title></div>";
        let (vault, uri) = vault_with(src);
        let off = src.find("th:title").unwrap() + 4; // just past "th:t"
        let items = completion(&vault, &uri, pos(off));
        assert!(!items.is_empty());
        // Filtered to names beginning with the typed prefix.
        assert!(items.iter().all(|i| i.label.starts_with("th:t")));
        assert!(items.iter().any(|i| i.label == "th:text"));
        assert!(items.iter().any(|i| i.label == "th:title"));
        // Non-matching attrs are excluded.
        assert!(!items.iter().any(|i| i.label == "th:if"));
    }

    #[test]
    fn text_context_returns_no_completions() {
        let src = "<p>Z</p>";
        let (vault, uri) = vault_with(src);
        let off = src.find('Z').unwrap();
        assert!(completion(&vault, &uri, pos(off)).is_empty());
    }
}
