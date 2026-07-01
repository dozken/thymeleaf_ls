//! Document links for Thymeleaf templates.
//!
//! Surfaces clickable links for URL-bearing attributes so editors can offer
//! "follow link" affordances. Two shapes are recognised:
//!
//!   * Thymeleaf link expressions `@{ ... }` in *any* attribute value (most
//!     often `th:href` / `th:src` / `th:action`). The path inside the braces is
//!     extracted, dropping any `(params)` suffix.
//!   * Plain `href="..."` / `src="..."` attributes whose value is a concrete
//!     `http(s)` URL.
//!
//! Absolute web URLs (`http://` / `https://`) get a parsed [`Url`] target so the
//! link navigates externally. Site-absolute (`/...`) and relative paths cannot
//! be resolved to a filesystem/URL location here, so they carry no target but
//! still render as highlighted links (with a descriptive tooltip).

use tower_lsp::lsp_types::*;

use crate::document::{AttrOccurrence, Document};

/// Collect [`DocumentLink`]s for every URL-bearing attribute in `doc`.
pub fn document_links(doc: &Document) -> Vec<DocumentLink> {
    let mut out = Vec::new();
    for attr in doc.attributes() {
        // Thymeleaf `@{ ... }` link expressions in any attribute value.
        collect_thymeleaf_links(doc, &attr, &mut out);
        // Plain `href` / `src` with a concrete http(s) URL.
        if let Some(link) = plain_url_link(doc, &attr) {
            out.push(link);
        }
    }
    out
}

/// Scan an attribute value for `@{ ... }` link expressions, pushing a
/// [`DocumentLink`] for each path found.
fn collect_thymeleaf_links(doc: &Document, attr: &AttrOccurrence, out: &mut Vec<DocumentLink>) {
    let value = &attr.value;
    let base = attr.value_range.start;

    let mut search = 0usize;
    while let Some(rel) = value[search..].find("@{") {
        // Byte index within `value` where the path begins (just past `@{`).
        let open = search + rel + 2;
        let rest = &value[open..];
        // Find the `}` that closes this `@{`, accounting for nested braces such
        // as path template variables (`@{/o/{id}(...)}`).
        let mut depth = 1usize;
        let mut close_rel = None;
        for (i, ch) in rest.char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        close_rel = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(close_rel) = close_rel else { break };

        // Content between `@{` and `}`; the URL path stops at the `(` that
        // introduces the parameter list (only when it is not inside `{...}`).
        let inner = &value[open..open + close_rel];
        let path_end_rel = param_paren(inner).unwrap_or(inner.len());
        let raw = &inner[..path_end_rel];

        // Trim surrounding whitespace while keeping byte-accurate offsets.
        let lead = raw.len() - raw.trim_start().len();
        let path = raw.trim();
        if !path.is_empty() {
            let start = base + open + lead;
            let end = start + path.len();
            if let Some(link) = make_link(doc, start, end, path) {
                out.push(link);
            }
        }

        // Continue scanning past this expression.
        search = open + close_rel + 1;
    }
}

/// Byte index of the `(` that introduces the parameter list, ignoring any `(`
/// that appears inside a `{...}` path variable. Returns `None` if there is none.
fn param_paren(inner: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, ch) in inner.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            '(' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}

/// Build a [`DocumentLink`] for a plain `href` / `src` attribute whose value is
/// a concrete `http(s)` URL. Returns `None` for other attributes or values.
fn plain_url_link(doc: &Document, attr: &AttrOccurrence) -> Option<DocumentLink> {
    if attr.name != "href" && attr.name != "src" {
        return None;
    }
    let path = attr.value.trim();
    if !(path.starts_with("http://") || path.starts_with("https://")) {
        return None;
    }
    // Byte range of the trimmed URL within the source.
    let lead = attr.value.len() - attr.value.trim_start().len();
    let start = attr.value_range.start + lead;
    let end = start + path.len();
    make_link(doc, start, end, path)
}

/// Construct a [`DocumentLink`] spanning the byte range `[start, end)` for
/// `path`. For `http(s)` URLs the target is the parsed [`Url`] (link is dropped
/// if parsing fails); other paths get no target but a descriptive tooltip.
fn make_link(doc: &Document, start: usize, end: usize, path: &str) -> Option<DocumentLink> {
    let is_http = path.starts_with("http://") || path.starts_with("https://");
    let range = Range {
        start: doc.position_at(start),
        end: doc.position_at(end),
    };
    let (target, tooltip) = if is_http {
        (Some(Url::parse(path).ok()?), None)
    } else {
        (None, Some("Thymeleaf link expression".to_string()))
    };
    Some(DocumentLink {
        range,
        target,
        tooltip,
        data: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::Document;

    fn doc(s: &str) -> Document {
        Document::new(s.to_string())
    }

    #[test]
    fn thymeleaf_absolute_url_gets_http_target() {
        let src = "<a th:href=\"@{https://example.com/x}\">L</a>";
        let d = doc(src);
        let links = document_links(&d);
        assert_eq!(links.len(), 1);
        let link = &links[0];
        assert_eq!(
            link.target.as_ref().map(|u| u.as_str()),
            Some("https://example.com/x")
        );
        // The range should cover exactly the path text.
        let start = d.offset_at(link.range.start);
        let end = d.offset_at(link.range.end);
        assert_eq!(&src[start..end], "https://example.com/x");
    }

    #[test]
    fn thymeleaf_site_absolute_path_gets_untargeted_link() {
        let src = "<a th:href=\"@{/local/path}\">L</a>";
        let d = doc(src);
        let links = document_links(&d);
        assert_eq!(links.len(), 1);
        let link = &links[0];
        assert!(link.target.is_none());
        assert!(link.tooltip.is_some());
        let start = d.offset_at(link.range.start);
        let end = d.offset_at(link.range.end);
        assert_eq!(&src[start..end], "/local/path");
    }

    #[test]
    fn thymeleaf_link_drops_param_suffix() {
        let src = "<a th:href=\"@{/o/{id}(id=3)}\">L</a>";
        let d = doc(src);
        let links = document_links(&d);
        assert_eq!(links.len(), 1);
        let link = &links[0];
        let start = d.offset_at(link.range.start);
        let end = d.offset_at(link.range.end);
        assert_eq!(&src[start..end], "/o/{id}");
    }

    #[test]
    fn plain_href_http_url_gets_target() {
        let src = "<a href=\"https://example.com/y\">L</a>";
        let d = doc(src);
        let links = document_links(&d);
        assert_eq!(links.len(), 1);
        assert_eq!(
            links[0].target.as_ref().map(|u| u.as_str()),
            Some("https://example.com/y")
        );
    }

    #[test]
    fn no_link_expression_yields_none() {
        let src = "<a th:text=\"hi\">Z</a>";
        let d = doc(src);
        assert!(document_links(&d).is_empty());
    }
}
