use chumsky::span::Span;

/// Simple comment preservation strategy for MVP
/// This approach re-inserts comments based on their original line positions
use crate::trivia::scan_comments;

pub fn format_with_comments(formatted_content: &str, original_content: &str) -> String {
    // For MVP, we'll just prepend file-level comments
    let comments = scan_comments(original_content);

    if comments.is_empty() {
        return formatted_content.to_string();
    }

    let mut output = String::new();

    // Find comments that appear before any code
    let first_non_comment_line = original_content
        .lines()
        .position(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with("//")
        })
        .unwrap_or(0);

    // Add leading comments
    for comment in &comments {
        let comment_line = original_content[..comment.span.start()]
            .chars()
            .filter(|&c| c == '\n')
            .count();

        if comment_line < first_non_comment_line {
            output.push_str(&comment.text);
            output.push('\n');
        }
    }

    // Add the formatted content
    output.push_str(formatted_content);

    output
}
