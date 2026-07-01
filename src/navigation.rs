//! Goto-definition and find-references for Thymeleaf fragments.
//!
//! Fragment model:
//!   * `th:fragment="name(args)"` **defines** a fragment named `name`.
//!   * `th:insert`, `th:replace`, `th:include` **reference** a fragment; their
//!     values look like `~{template :: name}`, `template :: name`, or `:: name`.
//!
//! `goto` (from a reference) jumps to the matching definition(s); `references`
//! (from either a definition or a reference) lists every reference site plus the
//! definition(s).

use tower_lsp::lsp_types::{Location, Position, Range, Url};

use crate::vault::Vault;

/// Goto-definition: when the cursor sits inside a `th:insert`/`th:replace`/
/// `th:include` value, resolve the referenced fragment name and return the
/// locations of matching `th:fragment` definitions.
pub fn goto(vault: &Vault, uri: &Url, position: Position) -> Option<Vec<Location>> {
    let doc = vault.get(uri)?;
    let offset = doc.offset_at(position);

    // Find the fragment-reference attribute whose value the cursor is inside.
    let name = doc
        .attributes()
        .into_iter()
        .filter(|a| is_reference_attr(&a.name))
        .find(|a| offset >= a.value_range.start && offset <= a.value_range.end)
        .map(|a| parse_reference_name(&a.value))?;

    if name.is_empty() {
        return None;
    }

    let locations: Vec<Location> = vault
        .find_fragment_definitions(&name)
        .into_iter()
        .map(|(uri, range)| Location { uri, range })
        .collect();

    if locations.is_empty() {
        None
    } else {
        Some(locations)
    }
}

/// Find-references: determine the fragment name under the cursor (whether the
/// cursor is on a `th:fragment` definition or on a reference) and return every
/// reference site across the vault, plus the definition(s).
pub fn references(vault: &Vault, uri: &Url, position: Position) -> Vec<Location> {
    let Some(name) = fragment_name_at(vault, uri, position) else {
        return Vec::new();
    };
    if name.is_empty() {
        return Vec::new();
    }

    let mut out: Vec<Location> = Vec::new();

    // Definition sites.
    for (def_uri, range) in vault.find_fragment_definitions(&name) {
        out.push(Location {
            uri: def_uri,
            range,
        });
    }

    // Reference sites across the vault.
    out.extend(reference_locations(vault, &name));

    out
}

/// Determines the fragment name relevant to the cursor position: first checks
/// whether the cursor is on a `th:fragment` definition, then whether it is
/// inside a fragment-reference value.
fn fragment_name_at(vault: &Vault, uri: &Url, position: Position) -> Option<String> {
    let doc = vault.get(uri)?;
    let offset = doc.offset_at(position);

    for attr in doc.attributes() {
        let on_name = offset >= attr.name_range.start && offset <= attr.name_range.end;
        let on_value = offset >= attr.value_range.start && offset <= attr.value_range.end;
        if !(on_name || on_value) {
            continue;
        }
        if is_fragment_attr(&attr.name) {
            let name = parse_definition_name(&attr.value);
            if !name.is_empty() {
                return Some(name);
            }
        }
        if is_reference_attr(&attr.name) {
            let name = parse_reference_name(&attr.value);
            if !name.is_empty() {
                return Some(name);
            }
        }
    }

    None
}

/// Collects the LSP locations of every fragment-reference attribute across the
/// vault whose parsed fragment name equals `name`.
fn reference_locations(vault: &Vault, name: &str) -> Vec<Location> {
    let mut out = Vec::new();
    for uri in vault_uris(vault) {
        let Some(doc) = vault.get(&uri) else { continue };
        for attr in doc.attributes() {
            if !is_reference_attr(&attr.name) {
                continue;
            }
            if parse_reference_name(&attr.value) != name {
                continue;
            }
            let range = Range {
                start: doc.position_at(attr.value_range.start),
                end: doc.position_at(attr.value_range.end),
            };
            out.push(Location {
                uri: uri.clone(),
                range,
            });
        }
    }
    out
}

/// Every document URI known to the vault.
///
/// This must enumerate *all* documents (via [`Vault::uris`]), not just those
/// that define fragments: reference-only documents (e.g. a page that
/// `th:replace`s a fragment defined in a shared template) would otherwise be
/// invisible to find-references.
fn vault_uris(vault: &Vault) -> Vec<Url> {
    vault.uris().cloned().collect()
}

// === Attribute-name classification ========================================

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

// === Value parsing ========================================================

/// Extracts the fragment name from a `th:fragment` value, dropping any
/// parameter list, e.g. `"header(title)"` -> `"header"`.
fn parse_definition_name(value: &str) -> String {
    let trimmed = value.trim();
    let name = match trimmed.find('(') {
        Some(idx) => &trimmed[..idx],
        None => trimmed,
    };
    name.trim().to_string()
}

/// Extracts the referenced fragment name from a `th:insert`/`th:replace`/
/// `th:include` value. Handles the forms:
///   * `~{template :: name}`
///   * `template :: name`
///   * `:: name`
///   * `name` (bare fragment, no template selector)
/// and strips any trailing argument list (`name(args)`).
fn parse_reference_name(value: &str) -> String {
    // Strip an outer `~{ ... }` fragment-expression wrapper if present.
    let mut s = value.trim();
    if let Some(rest) = s.strip_prefix("~{") {
        s = rest.trim_start();
        s = s.strip_suffix('}').unwrap_or(s);
    }
    let s = s.trim();

    // The fragment selector is the segment after `::`; if there's no `::` the
    // whole thing is the fragment name (bare reference).
    let selector = match s.rfind("::") {
        Some(idx) => &s[idx + 2..],
        None => s,
    };

    // Drop any argument list: `name(args)` -> `name`. Also drop a trailing `}`
    // that may remain if the wrapper suffix wasn't cleanly stripped.
    let selector = selector.trim();
    let selector = selector.strip_suffix('}').unwrap_or(selector);
    let name = match selector.find('(') {
        Some(idx) => &selector[..idx],
        None => selector,
    };
    name.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reference_name_wrapped_fragment_expression() {
        assert_eq!(parse_reference_name("~{tpl :: frag}"), "frag");
    }

    #[test]
    fn parse_reference_name_bare_template_selector() {
        assert_eq!(parse_reference_name("template :: name"), "name");
    }

    #[test]
    fn parse_reference_name_selector_only() {
        assert_eq!(parse_reference_name(":: name"), "name");
    }

    #[test]
    fn parse_reference_name_strips_arguments() {
        assert_eq!(parse_reference_name("~{fragments :: header('Home')}"), "header");
    }

    #[test]
    fn parse_definition_name_drops_parameter_list() {
        assert_eq!(parse_definition_name("header(title)"), "header");
        assert_eq!(parse_definition_name("footer"), "footer");
    }

    #[test]
    fn references_finds_reference_only_documents() {
        use crate::vault::Vault;
        let def_uri = Url::parse("file:///fragments.html").unwrap();
        let ref_uri = Url::parse("file:///page.html").unwrap();
        let mut vault = Vault::new(None);
        vault.upsert(
            def_uri.clone(),
            "<div th:fragment=\"header\">h</div>".to_string(),
        );
        // This document ONLY references the fragment (defines nothing).
        vault.upsert(
            ref_uri.clone(),
            "<div th:replace=\"~{fragments :: header}\"></div>".to_string(),
        );
        // Cursor on the definition name.
        let doc = vault.get(&def_uri).unwrap();
        let off = "<div th:fragment=\"".len() + 1;
        let pos = doc.position_at(off);
        let locs = references(&vault, &def_uri, pos);
        assert!(
            locs.iter().any(|l| l.uri == ref_uri),
            "expected reference in page.html, got {:?}",
            locs
        );
    }

    #[test]
    fn attribute_classification() {
        assert!(is_fragment_attr("th:fragment"));
        assert!(is_fragment_attr("data-th-fragment"));
        assert!(is_reference_attr("th:insert"));
        assert!(is_reference_attr("th:replace"));
        assert!(is_reference_attr("th:include"));
        assert!(!is_reference_attr("th:fragment"));
    }
}
