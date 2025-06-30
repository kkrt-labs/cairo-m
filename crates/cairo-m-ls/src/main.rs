#![feature(let_chains)]
#![allow(clippy::option_if_let_else)]

use std::sync::{Arc, Mutex};

use cairo_m_compiler::db::CompilerDatabase;
use cairo_m_compiler_diagnostics::{
    Diagnostic as CairoDiagnostic, DiagnosticSeverity as CairoSeverity,
};
use cairo_m_compiler_parser::{SourceProgram, Upcast};
use cairo_m_compiler_semantic::db::validate_semantics;
use cairo_m_compiler_semantic::semantic_index::{semantic_index, DefinitionId};
use cairo_m_compiler_semantic::type_resolution::definition_semantic_type;
use cairo_m_compiler_semantic::types::{TypeData, TypeId};
use cairo_m_compiler_semantic::{DefinitionKind, SemanticDb};
use dashmap::DashMap;
use salsa::Setter;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

/// LSP Backend for Cairo-M
///
/// Uses Mutex for the database to ensure thread-safe access in async context.
/// The `source_files` map stores the Salsa input for each file, enabling
/// incremental compilation on file changes.
struct Backend {
    client: Client,
    db: Arc<Mutex<CompilerDatabase>>,
    source_files: Arc<DashMap<Url, SourceProgram>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            db: Arc::new(Mutex::new(CompilerDatabase::default())),
            source_files: Arc::new(DashMap::new()),
        }
    }

    /// Convert Cairo diagnostic to LSP diagnostic
    fn convert_diagnostic(&self, source: &str, diag: &CairoDiagnostic) -> Diagnostic {
        let severity = match diag.severity {
            CairoSeverity::Error => DiagnosticSeverity::ERROR,
            CairoSeverity::Warning => DiagnosticSeverity::WARNING,
            CairoSeverity::Info => DiagnosticSeverity::INFORMATION,
            CairoSeverity::Hint => DiagnosticSeverity::HINT,
        };

        // Convert byte offsets to line/column positions
        let start_pos = self.offset_to_position(source, diag.span.start);
        let end_pos = self.offset_to_position(source, diag.span.end);

        let range = Range {
            start: start_pos,
            end: end_pos,
        };

        Diagnostic {
            range,
            severity: Some(severity),
            code: None,
            code_description: None,
            source: Some("cairo-m".to_string()),
            message: diag.message.clone(),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    /// Convert byte offset to LSP Position
    fn offset_to_position(&self, source: &str, offset: usize) -> Position {
        let mut line = 0;
        let mut character = 0;

        for (i, ch) in source.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                character = 0;
            } else {
                character += 1;
            }
        }

        Position {
            line: line as u32,
            character: character as u32,
        }
    }

    /// Convert LSP Position to byte offset
    fn position_to_offset(&self, source: &str, position: Position) -> usize {
        let mut current_line = 0;
        let mut current_character = 0;

        for (i, ch) in source.char_indices() {
            if current_line == position.line as usize
                && current_character == position.character as usize
            {
                return i;
            }

            if ch == '\n' {
                current_line += 1;
                current_character = 0;

                // If we're past the target line, the position doesn't exist
                if current_line > position.line as usize {
                    return source.len();
                }
            } else {
                current_character += 1;
            }
        }

        source.len()
    }

    /// Format a type for display in hover information
    fn format_type(db: &dyn SemanticDb, type_id: TypeId) -> String {
        match type_id.data(db) {
            TypeData::Felt => "felt".to_string(),
            TypeData::Pointer(inner) => format!("{}*", Self::format_type(db, inner)),
            TypeData::Tuple(types) => {
                if types.is_empty() {
                    "()".to_string()
                } else {
                    let formatted_types: Vec<String> =
                        types.iter().map(|t| Self::format_type(db, *t)).collect();
                    format!("({})", formatted_types.join(", "))
                }
            }
            TypeData::Function(_sig_id) => {
                // For now, just show "function" - we'd need to query the signature data
                "function".to_string()
            }
            TypeData::Struct(struct_id) => struct_id.name(db),
            TypeData::Unknown => "?".to_string(),
            TypeData::Error => "error".to_string(),
        }
    }

    /// Run semantic validation and publish diagnostics.
    async fn run_diagnostics(&self, uri: Url, version: Option<i32>) {
        if let Some(source) = self.source_files.get(&uri).map(|s| *s.value()) {
            let diagnostics = {
                let content;
                let semantic_diagnostics;
                {
                    let db = self.db.lock().unwrap();
                    content = source.text(&*db).to_owned();
                    semantic_diagnostics = validate_semantics(&*db, source);
                }

                semantic_diagnostics
                    .iter()
                    .map(|d| self.convert_diagnostic(&content, d))
                    .collect()
            };

            self.client
                .publish_diagnostics(uri, diagnostics, version)
                .await;
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions::default()),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Cairo-M language server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let content = params.text_document.text;
        let version = params.text_document.version;

        let path = match uri.to_file_path() {
            Ok(p) => p,
            Err(_) => return,
        };

        // Create a new SourceProgram for the opened file. This requires a mutable
        // database lock because it allocates a new entity.
        let source = {
            let db = self.db.lock().unwrap();
            SourceProgram::new(&*db, content, path.display().to_string())
        };
        self.source_files.insert(uri.clone(), source);

        self.run_diagnostics(uri, Some(version)).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;

        if let Some(change) = params.content_changes.into_iter().next() {
            // Update the SourceProgram with new content. This is the key to
            // incremental compilation.
            if let Some(source) = self.source_files.get(&uri).map(|s| *s.value()) {
                let mut db = self.db.lock().unwrap();
                source.set_text(&mut *db).to(change.text);
            }
            self.run_diagnostics(uri, Some(version)).await;
        }
    }

    #[allow(clippy::significant_drop_tightening)]
    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // Retrieve the SourceProgram from our map, do not create a new one.
        let source = match self.source_files.get(&uri) {
            Some(entry) => *entry.value(),
            None => return Ok(None),
        };

        let definition_location = {
            // Lock the DB immutably for querying.
            let db = self.db.lock().unwrap();
            let content = source.text(&*db);
            let offset = self.position_to_offset(content, position);

            // Run semantic analysis query. Salsa will compute it incrementally.
            let index = match semantic_index((*db).upcast(), source) {
                Ok(idx) => idx,
                Err(_) => return Ok(None),
            };

            // Find the identifier at the cursor position.
            let identifier_usage = index
                .identifier_usages()
                .iter()
                .enumerate()
                .find(|(_, usage)| usage.span.start <= offset && offset <= usage.span.end);

            if let Some((usage_idx, _)) = identifier_usage {
                // Get the definition for this usage.
                if let Some(definition) = index.get_use_definition(usage_idx) {
                    // Convert definition span to LSP location.
                    let start_pos = self.offset_to_position(content, definition.name_span.start);
                    let end_pos = self.offset_to_position(content, definition.name_span.end);

                    Some(Location {
                        uri,
                        range: Range {
                            start: start_pos,
                            end: end_pos,
                        },
                    })
                } else {
                    None
                }
            } else {
                None
            }
        };

        Ok(definition_location.map(GotoDefinitionResponse::Scalar))
    }

    #[allow(clippy::significant_drop_tightening)]
    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // Retrieve the SourceProgram from our map.
        let source = match self.source_files.get(&uri) {
            Some(entry) => *entry.value(),
            None => return Ok(None),
        };

        let hover_info = {
            let db = self.db.lock().unwrap();
            let content = source.text(&*db);
            let offset = self.position_to_offset(content, position);

            // Run semantic analysis query incrementally.
            let index = match semantic_index((*db).upcast(), source) {
                Ok(idx) => idx,
                Err(_) => return Ok(None),
            };

            let identifier_usage = index
                .identifier_usages()
                .iter()
                .enumerate()
                .find(|(_, usage)| usage.span.start <= offset && offset <= usage.span.end);

            if let Some((_usage_idx, usage)) = identifier_usage {
                if let Some((def_idx, _)) =
                    index.resolve_name_to_definition(&usage.name, usage.scope_id)
                {
                    let def_id = DefinitionId::new(&*db, source, def_idx);
                    let type_id = definition_semantic_type((*db).upcast(), def_id);
                    let type_str = Self::format_type((*db).upcast(), type_id);

                    let hover_text = format!("```cairo-m\n{}: {}\n```", usage.name, type_str);

                    Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: hover_text,
                        }),
                        range: None,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        };

        Ok(hover_info)
    }

    #[allow(clippy::significant_drop_tightening)]
    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        // Retrieve the SourceProgram from our map.
        let source = match self.source_files.get(&uri) {
            Some(entry) => *entry.value(),
            None => return Ok(None),
        };

        let completion_items = {
            let db = self.db.lock().unwrap();
            let content = source.text(&*db);
            let offset = self.position_to_offset(content, position);

            let index = match semantic_index((*db).upcast(), source) {
                Ok(idx) => idx,
                Err(_) => return Ok(None),
            };

            // Find the scope at the cursor position.
            // TODO: This is inefficient. A better approach would be to have a query
            // or a method on SemanticIndex to find the narrowest scope at an offset.
            let current_scope = {
                let mut best_scope = index.root_scope();
                let mut smallest_span_size = usize::MAX;

                for (scope_id, _) in index.scopes() {
                    for (_, def) in index.definitions_in_scope(scope_id) {
                        if def.full_span.start <= offset && offset <= def.full_span.end {
                            let span_size = def.full_span.end - def.full_span.start;
                            if span_size < smallest_span_size {
                                smallest_span_size = span_size;
                                best_scope = Some(scope_id);
                            }
                        }
                    }
                }

                best_scope.unwrap_or_else(|| index.root_scope().unwrap())
            };

            let mut items = Vec::new();
            let mut seen_names = std::collections::HashSet::new();
            let mut scope = Some(current_scope);

            while let Some(scope_id) = scope {
                if let Some(place_table) = index.place_table(scope_id) {
                    for (_, place) in place_table.places() {
                        let name = place.name.clone();
                        if seen_names.insert(name.clone()) {
                            if let Some((def_idx, def)) =
                                index.resolve_name_to_definition(&name, scope_id)
                            {
                                let type_str = {
                                    let db = self.db.lock().unwrap();
                                    let def_id = DefinitionId::new(&*db, source, def_idx);
                                    let type_id = definition_semantic_type((*db).upcast(), def_id);
                                    Self::format_type((*db).upcast(), type_id)
                                };

                                let kind = match def.kind {
                                    DefinitionKind::Function(_) => CompletionItemKind::FUNCTION,
                                    DefinitionKind::Parameter(_) => CompletionItemKind::VARIABLE,
                                    DefinitionKind::Local(_) => CompletionItemKind::VARIABLE,
                                    DefinitionKind::Let(_) => CompletionItemKind::VARIABLE,
                                    DefinitionKind::Const(_) => CompletionItemKind::CONSTANT,
                                    DefinitionKind::Struct(_) => CompletionItemKind::STRUCT,
                                    DefinitionKind::Import(_) => CompletionItemKind::MODULE,
                                    DefinitionKind::Namespace(_) => CompletionItemKind::MODULE,
                                    DefinitionKind::LoopVariable(_) => CompletionItemKind::VARIABLE,
                                };

                                items.push(CompletionItem {
                                    label: name,
                                    kind: Some(kind),
                                    detail: Some(type_str),
                                    documentation: None,
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
                scope = index.scope(scope_id).and_then(|s| s.parent);
            }

            // Add keywords
            let keywords = vec![
                ("if", CompletionItemKind::KEYWORD),
                ("else", CompletionItemKind::KEYWORD),
                ("while", CompletionItemKind::KEYWORD),
                ("loop", CompletionItemKind::KEYWORD),
                ("for", CompletionItemKind::KEYWORD),
                ("break", CompletionItemKind::KEYWORD),
                ("continue", CompletionItemKind::KEYWORD),
                ("return", CompletionItemKind::KEYWORD),
                ("let", CompletionItemKind::KEYWORD),
                ("local", CompletionItemKind::KEYWORD),
                ("const", CompletionItemKind::KEYWORD),
                ("func", CompletionItemKind::KEYWORD),
                ("struct", CompletionItemKind::KEYWORD),
                ("true", CompletionItemKind::KEYWORD),
                ("false", CompletionItemKind::KEYWORD),
                ("felt", CompletionItemKind::KEYWORD),
            ];

            for (keyword, kind) in keywords {
                items.push(CompletionItem {
                    label: keyword.to_string(),
                    kind: Some(kind),
                    ..Default::default()
                });
            }

            items
        };

        Ok(Some(CompletionResponse::Array(completion_items)))
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(Backend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
