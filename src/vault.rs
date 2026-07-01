//! Workspace / document store: holds parsed [`Document`]s keyed by URI and
//! provides a fragment index for navigation features.

use std::collections::HashMap;
use std::path::PathBuf;

use tower_lsp::lsp_types::{Range, Url};
use walkdir::WalkDir;

use crate::document::Document;

/// A `th:fragment` definition discovered somewhere in the workspace.
#[derive(Debug, Clone)]
pub struct FragmentDef {
    /// The fragment name (without its parameter list).
    pub name: String,
    /// The document that defines it.
    pub uri: Url,
    /// The range of the fragment *value* (LSP coordinates).
    pub range: Range,
}

/// In-memory store of all open/known documents plus the workspace root.
pub struct Vault {
    docs: HashMap<Url, Document>,
    root: Option<PathBuf>,
}

impl Vault {
    /// Creates an empty vault rooted at `root` (if known).
    pub fn new(root: Option<PathBuf>) -> Vault {
        Vault {
            docs: HashMap::new(),
            root,
        }
    }

    /// The workspace root directory, if one was provided.
    pub fn root(&self) -> Option<&PathBuf> {
        self.root.as_ref()
    }

    /// Inserts or replaces the document for `uri`, (re)parsing `text`.
    pub fn upsert(&mut self, uri: Url, text: String) {
        match self.docs.get_mut(&uri) {
            Some(doc) => doc.update(text),
            None => {
                self.docs.insert(uri, Document::new(text));
            }
        }
    }

    /// Removes the document for `uri`, if present.
    pub fn remove(&mut self, uri: &Url) {
        self.docs.remove(uri);
    }

    /// Returns the document for `uri`, if present.
    pub fn get(&self, uri: &Url) -> Option<&Document> {
        self.docs.get(uri)
    }

    /// Iterator over every known document URI.
    pub fn uris(&self) -> impl Iterator<Item = &Url> {
        self.docs.keys()
    }

    /// All fragment definitions across every known document.
    pub fn all_fragment_defs(&self) -> Vec<FragmentDef> {
        let mut out = Vec::new();
        for (uri, doc) in &self.docs {
            for attr in doc.attributes() {
                if !is_fragment_attr(&attr.name) {
                    continue;
                }
                let frag_name = parse_fragment_name(&attr.value);
                if frag_name.is_empty() {
                    continue;
                }
                let range = Range {
                    start: doc.position_at(attr.value_range.start),
                    end: doc.position_at(attr.value_range.end),
                };
                out.push(FragmentDef {
                    name: frag_name,
                    uri: uri.clone(),
                    range,
                });
            }
        }
        out
    }

    /// Locates the definition(s) of the fragment named `name`.
    pub fn find_fragment_definitions(&self, name: &str) -> Vec<(Url, Range)> {
        self.all_fragment_defs()
            .into_iter()
            .filter(|f| f.name == name)
            .map(|f| (f.uri, f.range))
            .collect()
    }

    /// Best-effort walk of the workspace root for `.html` files, upserting each
    /// into the vault. Read/parse errors are ignored.
    pub fn scan_workspace_html(&mut self) {
        let Some(root) = self.root.clone() else {
            return;
        };
        for entry in WalkDir::new(&root)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let is_html = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("html"))
                .unwrap_or(false);
            if !is_html {
                continue;
            }
            if let (Ok(text), Ok(uri)) = (
                std::fs::read_to_string(path),
                Url::from_file_path(path),
            ) {
                self.upsert(uri, text);
            }
        }
    }
}

/// True if the (possibly `data-th-`) attribute name denotes `th:fragment`.
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
