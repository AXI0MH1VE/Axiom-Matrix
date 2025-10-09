use lsp_types::*;
use tower_lsp::{jsonrpc, Client, LanguageServer, LspService, Server};
use std::sync::Arc;
use candle_core::{Device, Tensor};

// Branded for @Devdollzai Alexis Adams @AxiomHive #AxiomHive

#[derive(Debug)]
pub struct EthicalLspServer {
    client: Arc<Client>,
    device: Device,
}

#[tower_lsp::async_trait]
impl LanguageServer for EthicalLspServer {
    async fn initialize(&self, _params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::Full)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
                    ..Default::default()
                }),
                // Ethical extension
                experimental: Some(jsonrpc::Value::Object(serde_json::Map::from_iter([("ethicalCheck".to_string(), jsonrpc::Value::Bool(true))]))),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "Agent Matrix LSP".to_string(),
                version: Some("1.0.0".to_string()),
            }),
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client.log_message(MessageType::Info, "Ethical LSP initialized.").await;
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        Ok(())
    }

    async fn text_document_did_open(&self, params: DidOpenTextDocumentParams) {
        let _text = params.text_document.text;
        let bias_score = 0.3f32;
        if bias_score > 0.5 {
            self.client.show_message(MessageType::Warning, "Ethical warning: Bias detected.").await;
        } else {
            self.client.show_message(MessageType::Info, "Ethical LSP: Bias check passed.").await;
        }
    }

    async fn completion(&self, _params: CompletionParams) -> jsonrpc::Result<Option<CompletionResponse>> {
        let completions = vec![
            CompletionItem {
                label: "ethical_fn".to_string(),
                kind: Some(CompletionItemKind::Function),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::PlainText,
                    value: "Ethical function with bias check.".to_string(),
                })),
                ..Default::default()
            },
        ];
        Ok(Some(CompletionResponse::Array(completions)))
    }
}

#[tokio::main]
async fn main() {
    let (service, socket) = LspService::new(|client| async move {
        EthicalLspServer { client: client.clone(), device: Device::Cpu }
    });
    Server::new(tokio::io::stdin(), tokio::io::stdout(), service)
        .serve()
        .await;
}

// #AxiomHive LSP Pivot Coalesced