use structured_logger::{async_json::new_writer, Builder};
use tokio::sync::RwLock;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

use vault::Vault;

mod code_actions;
mod completion;
mod diagnostics;
mod document;
mod folding;
mod fragmentref;
mod highlight;
mod hover;
mod links;
mod navigation;
mod rename;
mod semantic_tokens;
mod symbols;
mod thymeleaf;
mod vault;

struct Backend {
    client: Client,
    vault: RwLock<Vault>,
}

impl Backend {
    /// Recomputes and publishes diagnostics for a single document.
    async fn publish_diagnostics(&self, uri: Url, version: Option<i32>) {
        let diags = {
            let vault = self.vault.read().await;
            match vault.get(&uri) {
                Some(doc) => diagnostics::diagnostics(doc),
                None => Vec::new(),
            }
        };
        self.client.publish_diagnostics(uri, diags, version).await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Resolve the workspace root. `root_uri` is deprecated in the LSP spec
        // in favour of `workspace_folders`; prefer the first folder, fall back
        // to `root_uri`, and run rootless if the client provides neither (a
        // missing root must not fail initialization).
        let root_path = params
            .workspace_folders
            .as_ref()
            .and_then(|folders| folders.first())
            .map(|folder| folder.uri.clone())
            .or(params.root_uri)
            .and_then(|uri| uri.to_file_path().ok());

        // Build the vault rooted at the workspace directory and pre-index the
        // workspace HTML so cross-file fragment navigation works immediately.
        {
            let mut vault = self.vault.write().await;
            *vault = Vault::new(root_path);
            vault.scan_workspace_html();
        }

        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "thymeleaf_ls".into(),
                version: Some("0.0.1".into()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![":".to_string(), "\"".to_string()]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                document_highlight_provider: Some(OneOf::Left(true)),
                document_link_provider: Some(DocumentLinkOptions {
                    resolve_provider: Some(false),
                    work_done_progress_options: Default::default(),
                }),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: Default::default(),
                })),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: semantic_tokens::legend(),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: Some(false),
                            work_done_progress_options: Default::default(),
                        },
                    ),
                ),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        log::info!("initialized");
        self.client
            .log_message(MessageType::INFO, "thymeleaf_ls initialized")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        log::debug!("did_open: {}", params.text_document.uri);
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;
        {
            let mut vault = self.vault.write().await;
            vault.upsert(uri.clone(), params.text_document.text);
        }
        self.publish_diagnostics(uri, Some(version)).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        log::debug!("did_change: {}", params.text_document.uri);
        let uri = params.text_document.uri.clone();
        let version = params.text_document.version;
        // INCREMENTAL sync: apply each change in order. A change with no range
        // is a full-document replacement.
        {
            let mut vault = self.vault.write().await;
            for change in params.content_changes {
                vault.apply_change(uri.clone(), change.range, change.text);
            }
        }
        self.publish_diagnostics(uri, Some(version)).await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        log::debug!("did_close: {}", params.text_document.uri);
        let uri = params.text_document.uri;
        {
            let mut vault = self.vault.write().await;
            vault.remove(&uri);
        }
        // Clear diagnostics for the closed document.
        self.client.publish_diagnostics(uri, Vec::new(), None).await;
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let vault = self.vault.read().await;
        let items = completion::completion(&vault, &uri, position);
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let vault = self.vault.read().await;
        Ok(hover::hover(&vault, &uri, position))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let vault = self.vault.read().await;
        let result = navigation::goto(&vault, &uri, position);
        Ok(result.map(GotoDefinitionResponse::Array))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let vault = self.vault.read().await;
        let locations = navigation::references(&vault, &uri, position);
        Ok(Some(locations))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let vault = self.vault.read().await;
        let Some(doc) = vault.get(&uri) else {
            return Ok(None);
        };
        let symbols = symbols::document_symbols(doc, &uri);
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let vault = self.vault.read().await;
        Ok(Some(symbols::workspace_symbols(&vault, &params.query)))
    }

    async fn document_highlight(
        &self,
        params: DocumentHighlightParams,
    ) -> Result<Option<Vec<DocumentHighlight>>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let vault = self.vault.read().await;
        let Some(doc) = vault.get(&uri) else {
            return Ok(None);
        };
        Ok(Some(highlight::document_highlight(doc, position)))
    }

    async fn document_link(&self, params: DocumentLinkParams) -> Result<Option<Vec<DocumentLink>>> {
        let uri = params.text_document.uri;
        let vault = self.vault.read().await;
        let Some(doc) = vault.get(&uri) else {
            return Ok(None);
        };
        Ok(Some(links::document_links(doc)))
    }

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let uri = params.text_document.uri;
        let vault = self.vault.read().await;
        let Some(doc) = vault.get(&uri) else {
            return Ok(None);
        };
        Ok(Some(folding::folding_ranges(doc)))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let range = params.range;
        let vault = self.vault.read().await;
        let Some(doc) = vault.get(&uri) else {
            return Ok(None);
        };
        Ok(Some(code_actions::code_actions(doc, &uri, range)))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let position = params.position;
        let vault = self.vault.read().await;
        Ok(rename::prepare_rename(&vault, &uri, position).map(PrepareRenameResponse::Range))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let vault = self.vault.read().await;
        Ok(rename::rename(&vault, &uri, position, &params.new_name))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let vault = self.vault.read().await;
        let Some(doc) = vault.get(&uri) else {
            return Ok(None);
        };
        let tokens = semantic_tokens::semantic_tokens_full(doc);
        Ok(Some(SemanticTokensResult::Tokens(tokens)))
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

/// Parses the log level from `--level <LEVEL>` (defaults to `INFO`).
fn parse_log_level() -> String {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--level" => {
                if let Some(level) = args.next() {
                    return level;
                }
            }
            other => {
                if let Some(level) = other.strip_prefix("--level=") {
                    return level.to_string();
                }
            }
        }
    }
    "INFO".to_string()
}

#[tokio::main]
async fn main() {
    let level = parse_log_level();
    // Logs must go to stderr only; stdout is reserved for the LSP JSON-RPC channel.
    Builder::with_level(&level)
        .with_target_writer("*", new_writer(tokio::io::stderr()))
        .init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        vault: RwLock::new(Vault::new(None)),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}

#[cfg(test)]
mod integration_tests {
    //! End-to-end tests exercising the public feature entrypoints through a
    //! populated `Vault`, mirroring how the LSP handlers call them.
    use super::*;
    use crate::document::Document;

    fn url(path: &str) -> Url {
        Url::parse(&format!("file:///{path}")).unwrap()
    }

    /// Position of the cursor placed immediately after the first occurrence of
    /// `marker` in `text`, computed via the document`s own offset->Position map.
    fn pos_after(doc: &Document, text: &str, marker: &str) -> Position {
        let offset = text.find(marker).expect("marker present") + marker.len();
        doc.position_at(offset)
    }

    /// Position of the cursor inside the first occurrence of `marker`.
    fn pos_inside(doc: &Document, text: &str, marker: &str) -> Position {
        let offset = text.find(marker).expect("marker present") + 1;
        doc.position_at(offset)
    }

    #[test]
    fn completion_offers_thymeleaf_attrs_in_attr_name_context() {
        let src = "<p th:t></p>";
        let uri = url("c.html");
        let mut vault = Vault::new(None);
        vault.upsert(uri.clone(), src.to_string());
        let pos = pos_after(vault.get(&uri).unwrap(), src, "th:t");

        let items = completion::completion(&vault, &uri, pos);
        assert!(
            !items.is_empty(),
            "expected completions in attr-name context"
        );
        assert!(items.iter().all(|i| i.label.starts_with("th:")));
        assert!(items.iter().any(|i| i.label == "th:text"));
        // items carry docs the editor can render
        assert!(items.iter().all(|i| i.documentation.is_some()));
    }

    #[test]
    fn completion_empty_in_plain_text() {
        let src = "<p>hello</p>";
        let uri = url("t.html");
        let mut vault = Vault::new(None);
        vault.upsert(uri.clone(), src.to_string());
        let pos = pos_inside(vault.get(&uri).unwrap(), src, "hello");
        assert!(completion::completion(&vault, &uri, pos).is_empty());
    }

    #[test]
    fn hover_returns_docs_on_known_attribute() {
        let src = "<p th:text=\"x\">hi</p>";
        let uri = url("h.html");
        let mut vault = Vault::new(None);
        vault.upsert(uri.clone(), src.to_string());
        let pos = pos_inside(vault.get(&uri).unwrap(), src, "th:text");

        let hover = hover::hover(&vault, &uri, pos).expect("hover on known attr");
        match hover.contents {
            HoverContents::Markup(m) => assert!(!m.value.is_empty()),
            _ => panic!("expected markup hover contents"),
        }
    }

    #[test]
    fn diagnostics_flag_unknown_attr_only() {
        let uri = url("d.html");
        let mut vault = Vault::new(None);

        vault.upsert(uri.clone(), "<p th:bogus=\"x\">hi</p>".to_string());
        let diags = diagnostics::diagnostics(vault.get(&uri).unwrap());
        assert_eq!(diags.len(), 1, "unknown th:* should produce one diagnostic");
        assert_eq!(diags[0].severity, Some(DiagnosticSeverity::WARNING));
        assert!(diags[0].message.contains("th:bogus"));

        vault.upsert(uri.clone(), "<p th:text=\"x\">hi</p>".to_string());
        assert!(
            diagnostics::diagnostics(vault.get(&uri).unwrap()).is_empty(),
            "valid th:text must not be flagged"
        );
    }

    #[test]
    fn navigation_goto_and_references_span_files() {
        let frag_uri = url("fragments.html");
        let page_uri = url("page.html");
        let frag_src = "<div th:fragment=\"header\">Site header</div>";
        let page_src = "<div th:replace=\"~{fragments :: header}\">placeholder</div>";

        let mut vault = Vault::new(None);
        vault.upsert(frag_uri.clone(), frag_src.to_string());
        vault.upsert(page_uri.clone(), page_src.to_string());

        // goto from the reference in page.html resolves to the definition file
        let goto_pos = pos_inside(vault.get(&page_uri).unwrap(), page_src, "header}");
        let defs = navigation::goto(&vault, &page_uri, goto_pos).expect("goto resolves");
        assert!(
            defs.iter().any(|l| l.uri == frag_uri),
            "definition in fragments.html"
        );

        // references from the definition include the reference-only page.html
        let ref_pos = pos_inside(vault.get(&frag_uri).unwrap(), frag_src, "header\"");
        let refs = navigation::references(&vault, &frag_uri, ref_pos);
        assert!(
            refs.iter().any(|l| l.uri == page_uri),
            "usage site found across files"
        );
    }
}
