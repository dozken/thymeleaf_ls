//! Document / workspace symbol providers.
//!
//! Turns the parsed attributes of a [`Document`] into an LSP outline
//! (`textDocument/documentSymbol`) and exposes the vault's fragment index as
//! workspace symbols (`workspace/symbol`). Both features are read-only views
//! built on top of the foundation APIs in [`crate::document`] and
//! [`crate::vault`].

use tower_lsp::lsp_types::*;

use crate::document::Document;
use crate::vault::Vault;

/// Builds the outline for a single document.
///
/// Emits one [`DocumentSymbol`] per `th:fragment` definition
/// ([`SymbolKind::FUNCTION`]) and per element carrying an `id` attribute
/// ([`SymbolKind::FIELD`]). The list is flat (no nesting) and sorted by start
/// position. `uri` is accepted for symmetry with the LSP request handler; the
/// ranges are always document-local so it is not otherwise needed here.
pub fn document_symbols(doc: &Document, _uri: &Url) -> Vec<DocumentSymbol> {
    let mut out: Vec<DocumentSymbol> = Vec::new();

    for attr in doc.attributes() {
        if is_fragment_attr(&attr.name) {
            let name = parse_fragment_name(&attr.value);
            if name.is_empty() {
                continue;
            }
            // The whole attribute spans the name through the end of the value;
            // the selection range highlights just the attribute name.
            let range = Range {
                start: doc.position_at(attr.name_range.start),
                end: doc.position_at(attr.value_range.end),
            };
            let selection_range = Range {
                start: doc.position_at(attr.name_range.start),
                end: doc.position_at(attr.name_range.end),
            };
            out.push(symbol(
                name,
                Some("th:fragment".to_string()),
                SymbolKind::FUNCTION,
                range,
                selection_range,
            ));
        } else if attr.name.eq_ignore_ascii_case("id") {
            let id = attr.value.trim();
            if id.is_empty() {
                continue;
            }
            let range = Range {
                start: doc.position_at(attr.name_range.start),
                end: doc.position_at(attr.value_range.end),
            };
            let selection_range = Range {
                start: doc.position_at(attr.value_range.start),
                end: doc.position_at(attr.value_range.end),
            };
            out.push(symbol(
                format!("#{id}"),
                None,
                SymbolKind::FIELD,
                range,
                selection_range,
            ));
        }
    }

    out.sort_by_key(|s| (s.range.start.line, s.range.start.character));
    out
}

/// Returns every fragment definition in the vault whose name contains `query`
/// (case-insensitive). An empty `query` matches all fragments.
#[allow(deprecated)]
pub fn workspace_symbols(vault: &Vault, query: &str) -> Vec<SymbolInformation> {
    let needle = query.to_ascii_lowercase();
    vault
        .all_fragment_defs()
        .into_iter()
        .filter(|f| needle.is_empty() || f.name.to_ascii_lowercase().contains(&needle))
        .map(|f| SymbolInformation {
            name: f.name,
            kind: SymbolKind::FUNCTION,
            location: Location {
                uri: f.uri,
                range: f.range,
            },
            container_name: None,
            tags: None,
            deprecated: None,
        })
        .collect()
}

/// Constructs a flat [`DocumentSymbol`] with the shared field defaults.
#[allow(deprecated)]
fn symbol(
    name: String,
    detail: Option<String>,
    kind: SymbolKind,
    range: Range,
    selection_range: Range,
) -> DocumentSymbol {
    DocumentSymbol {
        name,
        detail,
        kind,
        tags: None,
        deprecated: None,
        range,
        selection_range,
        children: None,
    }
}

/// True if `name` denotes a `th:fragment` attribute (either spelling).
fn is_fragment_attr(name: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    lower == "th:fragment" || lower == "data-th-fragment"
}

/// Extracts the fragment name from a `th:fragment` value, dropping any
/// parameter list, e.g. `"header(title)"` -> `"header"`.
fn parse_fragment_name(value: &str) -> String {
    let trimmed = value.trim();
    let name = match trimmed.find('(') {
        Some(idx) => &trimmed[..idx],
        None => trimmed,
    };
    name.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn doc(s: &str) -> Document {
        Document::new(s.to_string())
    }

    fn uri() -> Url {
        Url::parse("file:///test.html").unwrap()
    }

    #[test]
    fn document_symbols_finds_fragment_and_id() {
        let src = r#"<div th:fragment="header(title)"><span id="logo">x</span></div>"#;
        let d = doc(src);
        let syms = document_symbols(&d, &uri());

        let frag = syms
            .iter()
            .find(|s| s.kind == SymbolKind::FUNCTION)
            .expect("fragment symbol present");
        assert_eq!(frag.name, "header");
        assert_eq!(frag.detail.as_deref(), Some("th:fragment"));

        let id = syms
            .iter()
            .find(|s| s.kind == SymbolKind::FIELD)
            .expect("id symbol present");
        assert_eq!(id.name, "#logo");

        assert_eq!(syms.len(), 2);
    }

    #[test]
    fn document_symbols_sorted_by_position() {
        let src = r#"<a id="first">1</a><b id="second">2</b>"#;
        let d = doc(src);
        let syms = document_symbols(&d, &uri());
        assert_eq!(syms.len(), 2);
        assert_eq!(syms[0].name, "#first");
        assert_eq!(syms[1].name, "#second");
        let a = &syms[0].range.start;
        let b = &syms[1].range.start;
        assert!((a.line, a.character) <= (b.line, b.character));
    }

    #[test]
    fn workspace_symbols_filters_by_query() {
        let mut vault = Vault::new(Some(PathBuf::from("/ws")));
        vault.upsert(
            Url::parse("file:///a.html").unwrap(),
            r#"<div th:fragment="header">h</div>"#.to_string(),
        );
        vault.upsert(
            Url::parse("file:///b.html").unwrap(),
            r#"<div th:fragment="footer">f</div>"#.to_string(),
        );

        // Empty query matches every fragment.
        assert_eq!(workspace_symbols(&vault, "").len(), 2);

        // Case-insensitive substring match.
        let hits = workspace_symbols(&vault, "HEAD");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "header");
        assert_eq!(hits[0].kind, SymbolKind::FUNCTION);

        assert!(workspace_symbols(&vault, "nope").is_empty());
    }
}
