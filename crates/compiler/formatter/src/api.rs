use cairo_m_compiler_parser::parser::ParsedModule;
use cairo_m_compiler_parser::SourceFile;

use crate::comment_attachment::attach_comments_to_ast;
use crate::config::FormatterConfig;
use crate::context::FormatterCtx;
use crate::simple_comment_preserver::format_with_comments;
use crate::Format;

/// Format a source file
pub fn format_source_file(
    db: &dyn cairo_m_compiler_parser::Db,
    source: SourceFile,
    cfg: &FormatterConfig,
) -> String {
    let source_text = source.text(db);
    let parsed = cairo_m_compiler_parser::parse_file(db, source);

    if !parsed.diagnostics.is_empty() {
        return source_text.to_string();
    }

    format_parsed_module(db, &parsed.module, source_text, cfg)
}

/// Format a parsed module
pub fn format_parsed_module(
    _db: &dyn cairo_m_compiler_parser::Db,
    module: &ParsedModule,
    original_text: &str,
    cfg: &FormatterConfig,
) -> String {
    let mut ctx = FormatterCtx::new(cfg, original_text);

    // Attach comments to AST nodes (excludes file-level comments)
    let comment_buckets = attach_comments_to_ast(module, original_text);
    ctx.set_comments(comment_buckets);

    let doc = module.format(&mut ctx);
    let formatted = doc.render(cfg.max_width);

    // Apply file-level comment preservation
    format_with_comments(&formatted, original_text)
}

/// Format a range within a source file
pub fn format_range(
    db: &dyn cairo_m_compiler_parser::Db,
    source: SourceFile,
    _byte_start: usize,
    _byte_end: usize,
    cfg: &FormatterConfig,
) -> String {
    // MVP: Format the whole file
    format_source_file(db, source, cfg)
}
