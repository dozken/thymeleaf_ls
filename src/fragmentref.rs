//! Shared parsing for Thymeleaf fragment attributes.
//!
//! `th:fragment="name(args)"` *defines* a fragment; `th:insert` / `th:replace` /
//! `th:include` *reference* one via forms like `~{template :: name}`,
//! `template :: name`, `:: name`, or a bare `name`. Several features
//! (navigation, rename, highlight) need to recognise these attributes and
//! locate the fragment *name token* inside the attribute value, so that logic
//! lives here once instead of being copied per feature.
//!
//! The `*_range` functions return the byte range of the name token *within the
//! value string*; the `*_name` helpers return the corresponding slice.

use std::ops::Range;

/// True if `name` denotes a `th:fragment` definition (accepts `data-th-`).
pub fn is_fragment_attr(name: &str) -> bool {
    matches_th_attr(name, "fragment")
}

/// True if `name` denotes a fragment-reference attribute (`th:insert`,
/// `th:replace`, `th:include`; accepts the `data-th-` form).
pub fn is_reference_attr(name: &str) -> bool {
    matches_th_attr(name, "insert")
        || matches_th_attr(name, "replace")
        || matches_th_attr(name, "include")
}

/// Case-insensitively matches `name` against `th:<local>` or `data-th-<local>`.
fn matches_th_attr(name: &str, local: &str) -> bool {
    let lower = name.trim().to_ascii_lowercase();
    lower == format!("th:{local}") || lower == format!("data-th-{local}")
}

/// Trims ASCII whitespace from both ends of the window `[lo, hi)` of `s`.
fn trim_window(s: &str, mut lo: usize, mut hi: usize) -> (usize, usize) {
    let bytes = s.as_bytes();
    while lo < hi && bytes[lo].is_ascii_whitespace() {
        lo += 1;
    }
    while hi > lo && bytes[hi - 1].is_ascii_whitespace() {
        hi -= 1;
    }
    (lo, hi)
}

/// Byte range (within `value`) of the name token of a `th:fragment` value,
/// e.g. `"header(title)"` -> the range spanning `header`. `None` if empty.
pub fn definition_name_range(value: &str) -> Option<Range<usize>> {
    let (lo, hi) = trim_window(value, 0, value.len());
    let end = match value[lo..hi].find('(') {
        Some(idx) => lo + idx,
        None => hi,
    };
    let (lo, hi) = trim_window(value, lo, end);
    if lo >= hi {
        None
    } else {
        Some(lo..hi)
    }
}

/// Byte range (within `value`) of the referenced fragment name token of a
/// `th:insert`/`th:replace`/`th:include` value. Handles `~{tpl :: name}`,
/// `tpl :: name`, `:: name`, and bare `name`, stripping any argument list.
pub fn reference_name_range(value: &str) -> Option<Range<usize>> {
    let (mut lo, mut hi) = trim_window(value, 0, value.len());

    // Strip an outer `~{ ... }` fragment-expression wrapper if present.
    if value[lo..hi].starts_with("~{") {
        lo += 2;
        let t = trim_window(value, lo, hi);
        lo = t.0;
        hi = t.1;
        if value[lo..hi].ends_with('}') {
            hi -= 1;
        }
        let t = trim_window(value, lo, hi);
        lo = t.0;
        hi = t.1;
    }

    // The fragment selector is the segment after the last `::`.
    if let Some(idx) = value[lo..hi].rfind("::") {
        lo += idx + 2;
    }
    let t = trim_window(value, lo, hi);
    lo = t.0;
    hi = t.1;

    // Drop a stray trailing `}` and any argument list.
    if value[lo..hi].ends_with('}') {
        hi -= 1;
    }
    if let Some(idx) = value[lo..hi].find('(') {
        hi = lo + idx;
    }
    let (lo, hi) = trim_window(value, lo, hi);
    if lo >= hi {
        None
    } else {
        Some(lo..hi)
    }
}

/// The fragment name defined by a `th:fragment` value, e.g.
/// `"header(title)"` -> `Some("header")`.
pub fn definition_name(value: &str) -> Option<&str> {
    definition_name_range(value).map(|r| &value[r])
}

/// The fragment name referenced by a `th:insert`/`th:replace`/`th:include`
/// value, e.g. `"~{tpl :: header('Home')}"` -> `Some("header")`.
pub fn reference_name(value: &str) -> Option<&str> {
    reference_name_range(value).map(|r| &value[r])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_attributes() {
        assert!(is_fragment_attr("th:fragment"));
        assert!(is_fragment_attr("data-th-fragment"));
        assert!(is_reference_attr("th:replace"));
        assert!(is_reference_attr("TH:INSERT"));
        assert!(!is_reference_attr("th:text"));
        assert!(!is_fragment_attr("th:insert"));
    }

    #[test]
    fn definition_name_drops_parameter_list() {
        assert_eq!(definition_name("header(title)"), Some("header"));
        assert_eq!(definition_name("footer"), Some("footer"));
        assert_eq!(definition_name("  spaced  "), Some("spaced"));
        assert_eq!(definition_name("   "), None);
    }

    #[test]
    fn reference_name_handles_all_forms() {
        assert_eq!(reference_name("~{tpl :: frag}"), Some("frag"));
        assert_eq!(reference_name("template :: name"), Some("name"));
        assert_eq!(reference_name(":: name"), Some("name"));
        assert_eq!(reference_name("bare"), Some("bare"));
        assert_eq!(
            reference_name("~{fragments :: header('Home')}"),
            Some("header")
        );
    }

    #[test]
    fn ranges_point_at_the_name_token() {
        let v = "~{tpl :: header}";
        let r = reference_name_range(v).unwrap();
        assert_eq!(&v[r], "header");

        let v = "header(title)";
        let r = definition_name_range(v).unwrap();
        assert_eq!(&v[r], "header");
    }
}
