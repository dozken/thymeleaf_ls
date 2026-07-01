//! Static catalog of the Thymeleaf Standard Dialect.
//!
//! This module is pure data + lookup helpers. It is consumed by the feature
//! modules (completion / hover) to provide attribute names, human-readable
//! summaries and markdown documentation, plus a small reference of the
//! Thymeleaf expression syntaxes and utility objects.

/// A single Thymeleaf standard-dialect attribute.
///
/// `name` is always stored in the canonical `th:xxx` form. Lookups accept both
/// the `th:xxx` and the HTML5-valid `data-th-xxx` forms (see [`lookup`]).
#[derive(Debug, Clone, Copy)]
pub struct ThymeleafAttr {
    /// Canonical attribute name, e.g. `"th:text"`.
    pub name: &'static str,
    /// One-line summary suitable for a completion detail / signature line.
    pub summary: &'static str,
    /// Markdown documentation, including a short usage example.
    pub doc: &'static str,
}

/// The full catalog of standard-dialect attributes.
static ATTRS: &[ThymeleafAttr] = &[
    ThymeleafAttr {
        name: "th:text",
        summary: "Sets the escaped text content of an element.",
        doc: "Sets the (HTML-escaped) text body of the tag, replacing its content.\n\n```html\n<p th:text=\"${user.name}\">placeholder</p>\n```",
    },
    ThymeleafAttr {
        name: "th:utext",
        summary: "Sets the unescaped text content of an element.",
        doc: "Sets the tag body to *unescaped* text — raw HTML is rendered as markup.\nUse with care (XSS risk).\n\n```html\n<div th:utext=\"${htmlSnippet}\">...</div>\n```",
    },
    ThymeleafAttr {
        name: "th:if",
        summary: "Renders the element only if the expression is true.",
        doc: "Conditionally includes the element (and its content) when the expression\nevaluates truthy.\n\n```html\n<span th:if=\"${user.admin}\">Admin</span>\n```",
    },
    ThymeleafAttr {
        name: "th:unless",
        summary: "Renders the element only if the expression is false.",
        doc: "The negated counterpart of `th:if`.\n\n```html\n<span th:unless=\"${user.admin}\">Regular user</span>\n```",
    },
    ThymeleafAttr {
        name: "th:each",
        summary: "Iterates over a collection, repeating the element.",
        doc: "Iterates a collection/array/map/iterable, repeating the host tag for each\nitem. Supports an optional status variable.\n\n```html\n<li th:each=\"item : ${items}\" th:text=\"${item.name}\">item</li>\n```",
    },
    ThymeleafAttr {
        name: "th:switch",
        summary: "Selects one of several th:case branches.",
        doc: "Structural switch statement; used together with `th:case`.\n\n```html\n<div th:switch=\"${user.role}\">\n  <p th:case=\"'admin'\">Administrator</p>\n  <p th:case=\"*\">Unknown</p>\n</div>\n```",
    },
    ThymeleafAttr {
        name: "th:case",
        summary: "A branch of an enclosing th:switch.",
        doc: "Marks a case within a `th:switch`. `th:case=\"*\"` is the default branch.\n\n```html\n<p th:case=\"'admin'\">Administrator</p>\n```",
    },
    ThymeleafAttr {
        name: "th:object",
        summary: "Selects an object for *{...} selection expressions.",
        doc: "Sets the selection target so descendant `*{...}` expressions resolve\nagainst it.\n\n```html\n<div th:object=\"${user}\">\n  <span th:text=\"*{name}\">name</span>\n</div>\n```",
    },
    ThymeleafAttr {
        name: "th:with",
        summary: "Declares local variables for the element's scope.",
        doc: "Defines one or more local variables visible within the element subtree.\n\n```html\n<div th:with=\"total=${a + b}\">\n  <span th:text=\"${total}\">0</span>\n</div>\n```",
    },
    ThymeleafAttr {
        name: "th:attr",
        summary: "Sets arbitrary attributes from expressions.",
        doc: "Generic attribute setter; can set several attributes at once.\n\n```html\n<img th:attr=\"src=@{/img/logo.png},title=${title}\" />\n```",
    },
    ThymeleafAttr {
        name: "th:attrappend",
        summary: "Appends a value to an existing attribute.",
        doc: "Appends the computed value to the end of an attribute.\n\n```html\n<div class=\"base\" th:attrappend=\"class=${' ' + extra}\">...</div>\n```",
    },
    ThymeleafAttr {
        name: "th:attrprepend",
        summary: "Prepends a value to an existing attribute.",
        doc: "Prepends the computed value to the start of an attribute.\n\n```html\n<div class=\"base\" th:attrprepend=\"class=${extra + ' '}\">...</div>\n```",
    },
    ThymeleafAttr {
        name: "th:value",
        summary: "Sets the value attribute.",
        doc: "Shorthand for setting the `value` attribute of an input.\n\n```html\n<input type=\"text\" th:value=\"${user.name}\" />\n```",
    },
    ThymeleafAttr {
        name: "th:href",
        summary: "Sets the href attribute (usually a link expression).",
        doc: "Sets `href`, typically with a link `@{...}` expression that handles\ncontext paths and URL parameters.\n\n```html\n<a th:href=\"@{/users/{id}(id=${user.id})}\">profile</a>\n```",
    },
    ThymeleafAttr {
        name: "th:src",
        summary: "Sets the src attribute.",
        doc: "Sets `src`, typically with a link `@{...}` expression.\n\n```html\n<img th:src=\"@{/images/logo.png}\" />\n```",
    },
    ThymeleafAttr {
        name: "th:action",
        summary: "Sets a form's action attribute.",
        doc: "Sets the `action` of a form, usually via a link `@{...}` expression.\n\n```html\n<form th:action=\"@{/login}\" method=\"post\">...</form>\n```",
    },
    ThymeleafAttr {
        name: "th:method",
        summary: "Sets a form's method attribute.",
        doc: "Sets the HTTP `method` of a form.\n\n```html\n<form th:method=\"${'post'}\">...</form>\n```",
    },
    ThymeleafAttr {
        name: "th:field",
        summary: "Binds an input to a form-backing bean field.",
        doc: "Binds the input to a property of the `th:object`/command bean, setting\n`id`, `name` and `value` (or checked/selected) automatically.\n\n```html\n<input type=\"text\" th:field=\"*{email}\" />\n```",
    },
    ThymeleafAttr {
        name: "th:errors",
        summary: "Renders validation errors for a bound field.",
        doc: "Outputs the accumulated validation error messages for a field.\n\n```html\n<span th:errors=\"*{email}\">error</span>\n```",
    },
    ThymeleafAttr {
        name: "th:errorclass",
        summary: "Adds a CSS class when the bound field has errors.",
        doc: "Appends the given CSS class to the element only if the field has errors.\n\n```html\n<input th:field=\"*{email}\" th:errorclass=\"invalid\" />\n```",
    },
    ThymeleafAttr {
        name: "th:fragment",
        summary: "Declares a reusable fragment.",
        doc: "Defines a named, optionally parameterized fragment for reuse via\n`th:insert`/`th:replace`.\n\n```html\n<div th:fragment=\"header(title)\">\n  <h1 th:text=\"${title}\">Title</h1>\n</div>\n```",
    },
    ThymeleafAttr {
        name: "th:insert",
        summary: "Inserts a fragment inside the host tag.",
        doc: "Inserts the referenced fragment as *content* of the host tag (host tag is\nkept).\n\n```html\n<div th:insert=\"~{fragments :: header}\"></div>\n```",
    },
    ThymeleafAttr {
        name: "th:replace",
        summary: "Replaces the host tag with a fragment.",
        doc: "Replaces the host tag entirely with the referenced fragment.\n\n```html\n<div th:replace=\"~{fragments :: header('Home')}\"></div>\n```",
    },
    ThymeleafAttr {
        name: "th:include",
        summary: "Includes a fragment's content (deprecated; use th:insert).",
        doc: "Includes the *contents* of a fragment. Deprecated since Thymeleaf 3 in\nfavour of `th:insert`.\n\n```html\n<div th:include=\"~{fragments :: header}\"></div>\n```",
    },
    ThymeleafAttr {
        name: "th:remove",
        summary: "Removes the element/content at render time.",
        doc: "Removes the tag, its body, or repeated occurrences. Values: `all`,\n`body`, `tag`, `all-but-first`, `none`.\n\n```html\n<tr th:remove=\"all\">prototype row</tr>\n```",
    },
    ThymeleafAttr {
        name: "th:class",
        summary: "Sets the class attribute.",
        doc: "Sets the `class` attribute from an expression.\n\n```html\n<div th:class=\"${active} ? 'on' : 'off'\">...</div>\n```",
    },
    ThymeleafAttr {
        name: "th:classappend",
        summary: "Appends CSS classes to the existing class attribute.",
        doc: "Appends computed CSS class(es) without discarding static ones.\n\n```html\n<div class=\"box\" th:classappend=\"${active} ? 'active'\">...</div>\n```",
    },
    ThymeleafAttr {
        name: "th:style",
        summary: "Sets the style attribute.",
        doc: "Sets the inline `style` attribute from an expression.\n\n```html\n<div th:style=\"'color:' + ${color}\">...</div>\n```",
    },
    ThymeleafAttr {
        name: "th:styleappend",
        summary: "Appends to the existing style attribute.",
        doc: "Appends computed inline styles without discarding static ones.\n\n```html\n<div style=\"margin:0\" th:styleappend=\"'color:' + ${color}\">...</div>\n```",
    },
    ThymeleafAttr {
        name: "th:id",
        summary: "Sets the id attribute.",
        doc: "Sets the `id` attribute from an expression.\n\n```html\n<div th:id=\"'row-' + ${item.id}\">...</div>\n```",
    },
    ThymeleafAttr {
        name: "th:name",
        summary: "Sets the name attribute.",
        doc: "Sets the `name` attribute from an expression.\n\n```html\n<input th:name=\"${fieldName}\" />\n```",
    },
    ThymeleafAttr {
        name: "th:selected",
        summary: "Fixed-value boolean attribute: selected.",
        doc: "Sets/removes the `selected` boolean attribute based on the expression.\n\n```html\n<option th:selected=\"${item.id == current}\">...</option>\n```",
    },
    ThymeleafAttr {
        name: "th:checked",
        summary: "Fixed-value boolean attribute: checked.",
        doc: "Sets/removes the `checked` boolean attribute based on the expression.\n\n```html\n<input type=\"checkbox\" th:checked=\"${user.active}\" />\n```",
    },
    ThymeleafAttr {
        name: "th:disabled",
        summary: "Fixed-value boolean attribute: disabled.",
        doc: "Sets/removes the `disabled` boolean attribute based on the expression.\n\n```html\n<button th:disabled=\"${!form.valid}\">Save</button>\n```",
    },
    ThymeleafAttr {
        name: "th:readonly",
        summary: "Fixed-value boolean attribute: readonly.",
        doc: "Sets/removes the `readonly` boolean attribute based on the expression.\n\n```html\n<input th:readonly=\"${locked}\" />\n```",
    },
    ThymeleafAttr {
        name: "th:multiple",
        summary: "Fixed-value boolean attribute: multiple.",
        doc: "Sets/removes the `multiple` boolean attribute based on the expression.\n\n```html\n<select th:multiple=\"${allowMany}\">...</select>\n```",
    },
    ThymeleafAttr {
        name: "th:placeholder",
        summary: "Sets the placeholder attribute.",
        doc: "Sets the `placeholder` attribute, often from a message expression.\n\n```html\n<input th:placeholder=\"#{form.email}\" />\n```",
    },
    ThymeleafAttr {
        name: "th:alt",
        summary: "Sets the alt attribute.",
        doc: "Sets the `alt` attribute of an image.\n\n```html\n<img th:alt=\"#{logo.alt}\" th:src=\"@{/logo.png}\" />\n```",
    },
    ThymeleafAttr {
        name: "th:title",
        summary: "Sets the title attribute.",
        doc: "Sets the `title` attribute.\n\n```html\n<a th:title=\"#{link.help}\">?</a>\n```",
    },
    ThymeleafAttr {
        name: "th:lang",
        summary: "Sets the lang attribute.",
        doc: "Sets the `lang` attribute.\n\n```html\n<html th:lang=\"${#locale.language}\">...</html>\n```",
    },
    ThymeleafAttr {
        name: "th:block",
        summary: "A synthetic container element (<th:block>).",
        doc: "The `th:block` element is a non-rendered container, useful to apply\niteration/conditions without introducing a wrapping tag.\n\n```html\n<th:block th:each=\"i : ${items}\">\n  <td th:text=\"${i}\">0</td>\n</th:block>\n```",
    },
    ThymeleafAttr {
        name: "th:inline",
        summary: "Enables inline expression processing in text/JS/CSS.",
        doc: "Enables inlined expressions (`[[...]]` / `[(...)]`) within the element.\nValues: `text`, `javascript`, `css`, `none`.\n\n```html\n<script th:inline=\"javascript\">\n  var user = /*[[${user.name}]]*/ 'default';\n</script>\n```",
    },
];

/// Returns the full catalog of standard-dialect attributes.
pub fn all_attrs() -> &'static [ThymeleafAttr] {
    ATTRS
}

/// Normalizes an attribute name to the canonical `th:xxx` form.
///
/// Accepts both `th:text` and the HTML5-valid `data-th-text` spellings.
/// Returns `None` if the name is not in a recognizable Thymeleaf form.
fn normalize(name: &str) -> Option<String> {
    let lower = name.trim().to_ascii_lowercase();
    if let Some(rest) = lower.strip_prefix("th:") {
        Some(format!("th:{}", rest))
    } else { lower.strip_prefix("data-th-").map(|rest| format!("th:{}", rest)) }
}

/// Looks up an attribute by name, accepting both `th:text` and
/// `data-th-text` spellings.
pub fn lookup(name: &str) -> Option<&'static ThymeleafAttr> {
    let canonical = normalize(name)?;
    ATTRS.iter().find(|a| a.name == canonical)
}

/// Reference data for the Thymeleaf expression syntaxes and common utility
/// objects. Each entry is `(token, markdown-description)`.
static EXPRESSION_SYNTAXES: &[(&str, &str)] = &[
    (
        "${...}",
        "**Variable expression** — evaluates against the context variables (the model).\n\n```html\n<span th:text=\"${user.name}\">name</span>\n```",
    ),
    (
        "*{...}",
        "**Selection expression** — evaluates against the current `th:object` selection.\n\n```html\n<div th:object=\"${user}\"><span th:text=\"*{name}\">name</span></div>\n```",
    ),
    (
        "#{...}",
        "**Message (i18n) expression** — resolves externalized text from message bundles.\n\n```html\n<p th:text=\"#{home.welcome}\">Welcome</p>\n```",
    ),
    (
        "@{...}",
        "**Link (URL) expression** — builds context-aware URLs and query parameters.\n\n```html\n<a th:href=\"@{/users/{id}(id=${id})}\">link</a>\n```",
    ),
    (
        "~{...}",
        "**Fragment expression** — references a fragment for inclusion.\n\n```html\n<div th:insert=\"~{footer :: copy}\"></div>\n```",
    ),
    (
        "#dates",
        "Utility object for `java.util.Date` formatting and creation.\n\n`${#dates.format(date, 'dd/MMM/yyyy HH:mm')}`",
    ),
    (
        "#calendars",
        "Utility object analogous to `#dates` but for `java.util.Calendar`.",
    ),
    (
        "#numbers",
        "Utility object for formatting numeric values.\n\n`${#numbers.formatDecimal(n, 3, 2)}`",
    ),
    (
        "#strings",
        "Utility object for `String` operations.\n\n`${#strings.isEmpty(name)}`, `${#strings.abbreviate(s, 10)}`",
    ),
    (
        "#objects",
        "Utility object for general object operations, e.g. defaults.\n\n`${#objects.nullSafe(obj, default)}`",
    ),
    (
        "#bools",
        "Utility object for boolean evaluation.\n\n`${#bools.isTrue(cond)}`",
    ),
    (
        "#arrays",
        "Utility object for arrays.\n\n`${#arrays.length(arr)}`, `${#arrays.contains(arr, x)}`",
    ),
    (
        "#lists",
        "Utility object for lists.\n\n`${#lists.size(list)}`, `${#lists.isEmpty(list)}`",
    ),
    (
        "#sets",
        "Utility object for sets.\n\n`${#sets.size(set)}`, `${#sets.contains(set, x)}`",
    ),
    (
        "#maps",
        "Utility object for maps.\n\n`${#maps.size(map)}`, `${#maps.containsKey(map, k)}`",
    ),
    (
        "#aggregates",
        "Utility object for aggregation over arrays/collections.\n\n`${#aggregates.sum(list)}`, `${#aggregates.avg(list)}`",
    ),
    (
        "#ids",
        "Utility object for generating unique `id` attributes in iterations.\n\n`${#ids.seq('itemId')}`",
    ),
];

/// Returns reference data for expression syntaxes and utility objects.
pub fn expression_syntaxes() -> &'static [(&'static str, &'static str)] {
    EXPRESSION_SYNTAXES
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_canonical_th_form() {
        let attr = lookup("th:text").expect("th:text should be in the catalog");
        assert_eq!(attr.name, "th:text");
    }

    #[test]
    fn lookup_normalizes_data_th_form() {
        // The HTML5-valid `data-th-*` spelling resolves to the canonical name.
        let attr = lookup("data-th-text").expect("data-th-text should normalize to th:text");
        assert_eq!(attr.name, "th:text");
    }

    #[test]
    fn lookup_is_case_insensitive() {
        let attr = lookup("TH:TEXT").expect("lookup should be case-insensitive");
        assert_eq!(attr.name, "th:text");
    }

    #[test]
    fn lookup_unknown_returns_none() {
        assert!(lookup("th:bogus").is_none());
        assert!(lookup("class").is_none());
    }

    #[test]
    fn catalog_is_non_empty_and_canonical() {
        assert!(!all_attrs().is_empty());
        // Every catalog entry is stored in canonical `th:` form.
        assert!(all_attrs().iter().all(|a| a.name.starts_with("th:")));
    }
}
