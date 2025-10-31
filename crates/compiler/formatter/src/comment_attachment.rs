use cairo_m_compiler_parser::parser::{
    Expression, FunctionDef, ParsedModule, Statement, TopLevelItem,
};
use chumsky::span::SimpleSpan;

use crate::trivia::{
    Comment, CommentBuckets, CommentPosition, determine_comment_position, scan_comments,
};

/// Trait for AST nodes that have spans
pub trait HasSpan {
    fn span(&self) -> SimpleSpan<usize>;
}

// Implement for Spanned<T> from the parser
impl<T> HasSpan for cairo_m_compiler_parser::parser::Spanned<T> {
    fn span(&self) -> SimpleSpan<usize> {
        self.span()
    }
}

/// Attach comments to AST nodes based on their spans
pub fn attach_comments_to_ast(module: &ParsedModule, source: &str) -> CommentBuckets {
    let comments = scan_comments(source);
    let mut buckets = CommentBuckets::new();
    let mut node_spans = Vec::new();

    // Collect all node spans from the AST
    collect_node_spans(module, &mut node_spans);

    // Sort spans by start position for efficient attachment
    node_spans.sort_by_key(|span| span.start);

    // Find the first code position (non-comment, non-whitespace)
    let first_code_pos = source
        .lines()
        .enumerate()
        .find(|(_, line)| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//")
        })
        .map(|(line_num, _)| {
            source
                .lines()
                .take(line_num)
                .map(|l| l.len() + 1)
                .sum::<usize>()
                .saturating_sub(1)
        })
        .unwrap_or(0);

    // For each comment, find the best node to attach it to
    for comment in comments {
        // Skip file-level comments (before first code)
        if comment.span.start < first_code_pos {
            continue;
        }

        if let Some(node_span) = find_best_attachment(&comment, &node_spans, source) {
            match determine_comment_position(comment.span, node_span, source) {
                Some(CommentPosition::Before) => {
                    buckets.add_leading(node_span, comment);
                }
                Some(CommentPosition::EndOfLine) => {
                    buckets.add_trailing(node_span, comment);
                }
                Some(CommentPosition::After) => {
                    buckets.add_trailing(node_span, comment);
                }
                None => {
                    // Orphaned comment - attach to nearest node
                    if let Some(nearest) = find_nearest_node(&comment, &node_spans) {
                        buckets.add_leading(nearest, comment);
                    }
                }
            }
        }
    }

    buckets
}

/// Collect all spans from AST nodes
fn collect_node_spans(module: &ParsedModule, spans: &mut Vec<SimpleSpan<usize>>) {
    for item in &module.items {
        collect_item_spans(item, spans);
    }
}

fn collect_item_spans(item: &TopLevelItem, spans: &mut Vec<SimpleSpan<usize>>) {
    match item {
        TopLevelItem::Function(func) => {
            spans.push(func.span());
            collect_function_spans(func.value(), spans);
        }
        TopLevelItem::Struct(s) => {
            spans.push(s.span());
        }
        TopLevelItem::Const(c) => {
            spans.push(c.span());
        }
        TopLevelItem::Use(u) => {
            spans.push(u.span());
        }
    }
}

fn collect_function_spans(func: &FunctionDef, spans: &mut Vec<SimpleSpan<usize>>) {
    // Collect spans from function body statements
    for stmt in &func.body {
        collect_statement_spans(stmt, spans);
    }
}

fn collect_statement_spans(
    stmt: &cairo_m_compiler_parser::parser::Spanned<Statement>,
    spans: &mut Vec<SimpleSpan<usize>>,
) {
    spans.push(stmt.span());

    // Recursively collect from nested statements and expressions
    match stmt.value() {
        Statement::If {
            condition,
            then_block,
            else_block,
        } => {
            collect_expression_spans(condition, spans);
            collect_statement_spans(then_block, spans);
            if let Some(else_stmt) = else_block {
                collect_statement_spans(else_stmt, spans);
            }
        }
        Statement::While { condition, body } => {
            collect_expression_spans(condition, spans);
            collect_statement_spans(body, spans);
        }
        Statement::For {
            init,
            condition,
            step,
            body,
        } => {
            collect_statement_spans(init, spans);
            collect_expression_spans(condition, spans);
            collect_statement_spans(step, spans);
            collect_statement_spans(body, spans);
        }
        Statement::Block(statements) => {
            for s in statements {
                collect_statement_spans(s, spans);
            }
        }
        Statement::Let { value, .. } => {
            collect_expression_spans(value, spans);
        }
        Statement::Assignment { lhs, rhs } => {
            collect_expression_spans(lhs, spans);
            collect_expression_spans(rhs, spans);
        }
        Statement::Return { value: Some(expr) } => {
            collect_expression_spans(expr, spans);
        }
        Statement::Return { value: None } => {}
        Statement::Expression(expr) => {
            collect_expression_spans(expr, spans);
        }
        _ => {} // Other statement types
    }
}

fn collect_expression_spans(
    expr: &cairo_m_compiler_parser::parser::Spanned<Expression>,
    spans: &mut Vec<SimpleSpan<usize>>,
) {
    spans.push(expr.span());

    // Recursively collect from nested expressions
    match expr.value() {
        Expression::BinaryOp { left, right, .. } => {
            collect_expression_spans(left, spans);
            collect_expression_spans(right, spans);
        }
        Expression::UnaryOp { expr, .. } => {
            collect_expression_spans(expr, spans);
        }
        Expression::FunctionCall { callee, args } => {
            collect_expression_spans(callee, spans);
            for arg in args {
                collect_expression_spans(arg, spans);
            }
        }
        Expression::IndexAccess { array, index } => {
            collect_expression_spans(array, spans);
            collect_expression_spans(index, spans);
        }
        Expression::MemberAccess { object, .. } => {
            collect_expression_spans(object, spans);
        }
        Expression::Tuple(elements) => {
            for elem in elements {
                collect_expression_spans(elem, spans);
            }
        }
        Expression::Parenthesized(inner) => {
            collect_expression_spans(inner, spans);
        }
        Expression::StructLiteral { fields, .. } => {
            for (_, value) in fields {
                collect_expression_spans(value, spans);
            }
        }
        _ => {} // Literals and identifiers
    }
}

/// Find the best node span to attach a comment to
fn find_best_attachment(
    comment: &Comment,
    node_spans: &[SimpleSpan<usize>],
    source: &str,
) -> Option<SimpleSpan<usize>> {
    // Find nodes that could be related to this comment
    let mut candidates: Vec<(
        SimpleSpan<usize>,
        crate::trivia::CommentPosition,
        usize,
        u32,
    )> = Vec::new();

    for &span in node_spans {
        if let Some(pos) = determine_comment_position(comment.span, span, source) {
            // Distance from the relevant edge
            let distance = if comment.span.start < span.start {
                span.start.saturating_sub(comment.span.start)
            } else {
                comment.span.start.saturating_sub(span.end)
            };

            // Rank preferences: EndOfLine < Before < After
            // docstrings ("///") should strongly prefer Before
            let is_doc = comment.text.trim_start().starts_with("///");
            let rank = match pos {
                CommentPosition::EndOfLine => 0,
                CommentPosition::Before => {
                    if is_doc {
                        0
                    } else {
                        1
                    }
                }
                CommentPosition::After => {
                    if is_doc {
                        3
                    } else {
                        2
                    }
                }
            };
            candidates.push((span, pos, distance, rank));
        }
    }

    // Prefer lower rank then smaller distance
    candidates
        .into_iter()
        .min_by_key(|&(_span, _pos, distance, rank)| (rank, distance))
        .map(|(span, _pos, _distance, _rank)| span)
}

/// Find the nearest node to a comment
fn find_nearest_node(
    comment: &Comment,
    node_spans: &[SimpleSpan<usize>],
) -> Option<SimpleSpan<usize>> {
    node_spans
        .iter()
        .min_by_key(|&&span| {
            if comment.span.start < span.start {
                span.start - comment.span.start
            } else {
                comment.span.start - span.end
            }
        })
        .copied()
}
