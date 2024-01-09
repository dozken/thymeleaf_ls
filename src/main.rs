use std::ops::Deref;
use std::path::Path;

use structured_logger::{async_json::new_writer, unix_ms, Builder};
// use completion::get_completions;
// use references::references;
use tokio::sync::RwLock;

// use gotodef::goto_definition;
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
// use vault::{Vault, construct_vault, reconstruct_vault};

// mod vault;
// mod gotodef;
// mod references;
// mod completion;

#[derive(Debug)]
struct Backend {
    client: Client,
    // vault: RwLock<Option<Vault>>
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, i: InitializeParams) -> Result<InitializeResult> {
        let Some(root_uri) = i.root_uri else {
            return Err(Error::new(ErrorCode::InvalidParams));
        };
        let root_dir = Path::new(root_uri.path());

        return Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "thymeleaf_ls".into(),
                version: Some("0.0.1".into()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![
                        ":".to_string(),
                    ]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                // definition: Some(GotoCapability::default()),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
        });
    }

    async fn initialized(&self, _: InitializedParams) {
        log::info!("initialized");
        self.client
            .log_message(MessageType::INFO, "thymeleaf_ls initialized")
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        log::debug!("did_open: {:?}", params);
        // let Some(ref mut vault) = *self.vault.write().await else {
        //     self.client.log_message(MessageType::ERROR, "Vault is not initialized").await;
        //     return;
        // };
        //
        // let Ok(path) = params.text_document.uri.to_file_path() else {
        //     self.client.log_message(MessageType::ERROR, "Failed to parse URI path").await;
        //     return;
        // };
        // let text = params.text_document.text;
        // reconstruct_vault(vault, (&path, &text));
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        // log::debug!("did_change: {:?}", params);
        //
        // let Some(ref mut vault) = *self.vault.write().await else {
        //     self.client.log_message(MessageType::ERROR, "Vault is not initialized").await;
        //     return;
        // };
        //
        // let Ok(path) = params.text_document.uri.to_file_path() else {
        //     self.client.log_message(MessageType::ERROR, "Failed to parse URI path").await;
        //     return;
        // };
        // let text = &params.content_changes[0].text;
        // reconstruct_vault(vault, (&path, text));
    }

    // async fn goto_definition(
    //     &self,
    //     params: GotoDefinitionParams,
    // ) -> Result<Option<GotoDefinitionResponse>> {
    //
    //     let position = params.text_document_position_params.position;
    //
    //     let vault_option = self.vault.read().await;
    //     let Some(vault) = vault_option.deref() else {
    //         return Err(Error::new(ErrorCode::ServerError(0)));
    //     };
    //     let Ok(path) = params.text_document_position_params.text_document.uri.to_file_path() else {
    //         return Err(Error::new(ErrorCode::ServerError(0)));
    //     };
    //     let result = goto_definition(&vault, position, &path);
    //
    //
    //     return Ok(result.map(|l| GotoDefinitionResponse::Array(l)))
    // }

    // async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
    //     let position = params.text_document_position.position;
    //
    //     let vault_option = self.vault.read().await;
    //     let Some(vault) = vault_option.deref() else {
    //         return Err(Error::new(ErrorCode::ServerError(0)));
    //     };
    //     let Ok(path) = params.text_document_position.text_document.uri.to_file_path() else {
    //         return Err(Error::new(ErrorCode::ServerError(0)));
    //     };
    //
    //     let locations = references(vault, position, &path);
    //     Ok(locations)
    // }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        // let bad_vault = self.vault.read().await;
        // let Some(vault) = bad_vault.deref() else {
        //     return Err(Error::new(ErrorCode::ServerError(0)))
        // };
        // let completions = get_completions(vault, &params);
        // if completions == None {
        //     self.client.log_message(MessageType::INFO, format!("No completions for: {:?}", params)).await;
        // }
        // Ok(completions)
        //
        //
        // Ok(completions.map(CompletionResponse::Array))
        let completions = CompletionResponse::Array(vec![
            CompletionItem::new_simple("thyme".into(), "leaf".into()),
            CompletionItem::new_simple("hello".into(), "world".into()),
        ]);

        match &params.context {
            Some(context) => match &context.trigger_character {
                Some(trigger_character) => match trigger_character.as_str() {
                    ":" => {
                        log::debug!("colon {:?}", &params);

                        return Ok(Some(CompletionResponse::Array(vec![
                            CompletionItem::new_simple("thyme".into(), "leaf".into()),
                            CompletionItem::new_simple("hello".into(), "world".into()),
                        ])));
                    }
                    "-" => {
                        log::debug!("hyphen {:?}", &params);
                        return Ok(Some(CompletionResponse::Array(vec![
                            CompletionItem::new_simple("hyme".into(), "leaf".into()),
                            CompletionItem::new_simple("ello".into(), "world".into()),
                        ])));
                    }
                    tc => {
                        log::debug!("any {:?}", tc);
                        return Ok(Some(CompletionResponse::Array(vec![
                            CompletionItem::new_simple("me".into(), "leaf".into()),
                            CompletionItem::new_simple("lo".into(), "world".into()),
                        ])));
                    }
                },
                None => {}
            },
            None => {}
        }

        return Ok(None);
    }
}

#[tokio::main]
async fn main() {
    Builder::with_level("TRACE")
        .with_target_writer("*", new_writer(tokio::io::stderr()))
        .init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend { client });
    Server::new(stdin, stdout, socket).serve(service).await;
}
