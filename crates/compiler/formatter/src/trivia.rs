use std::collections::HashMap;

use chumsky::span::SimpleSpan;

#[derive(Debug, Clone)]
pub struct Comment {
    pub text: String,
    pub span: SimpleSpan<usize>,
    pub is_line: bool, // true for //, false for /* */
}

#[derive(Debug, Default)]
pub struct CommentBuckets {
    pub leading: HashMap<SimpleSpan<usize>, Vec<Comment>>,
    pub trailing: HashMap<SimpleSpan<usize>, Vec<Comment>>,
}

impl CommentBuckets {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_leading(&mut self, node_span: SimpleSpan<usize>, comment: Comment) {
        self.leading.entry(node_span).or_default().push(comment);
    }

    pub fn add_trailing(&mut self, node_span: SimpleSpan<usize>, comment: Comment) {
        self.trailing.entry(node_span).or_default().push(comment);
    }

    pub fn get_leading(&self, span: SimpleSpan<usize>) -> Option<&[Comment]> {
        self.leading.get(&span).map(|v| v.as_slice())
    }

    pub fn get_trailing(&self, span: SimpleSpan<usize>) -> Option<&[Comment]> {
        self.trailing.get(&span).map(|v| v.as_slice())
    }
}

/// Scan source text for comments
pub fn scan_comments(source: &str) -> Vec<Comment> {
    let mut comments = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        // Check for line comment
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            let start = i;
            i += 2;

            // Find end of line
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }

            let text = String::from_utf8_lossy(&bytes[start..i]).to_string();
            comments.push(Comment {
                text,
                span: SimpleSpan::from(start..i),
                is_line: true,
            });
        } else {
            i += 1;
        }
    }

    comments
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CommentPosition {
    Before,
    After,
    EndOfLine,
}

/// Determine where a comment should be attached relative to an AST node
pub fn determine_comment_position(
    comment_span: SimpleSpan<usize>,
    node_span: SimpleSpan<usize>,
    source: &str,
) -> Option<CommentPosition> {
    let comment_start = comment_span.start;
    let node_start = node_span.start;
    let node_end = node_span.end;

    if comment_start < node_start {
        // Check if comment is on the line immediately before the node
        let between = &source[comment_start..node_start];
        let newlines = between.chars().filter(|&c| c == '\n').count();
        if newlines <= 1 {
            return Some(CommentPosition::Before);
        }
    } else if comment_start >= node_end {
        // Check if comment is on the same line as the node end
        let before_comment = &source[node_end..comment_start];
        if !before_comment.contains('\n') {
            return Some(CommentPosition::EndOfLine);
        }
        // Check if it's on the next line
        let newlines = before_comment.chars().filter(|&c| c == '\n').count();
        if newlines <= 1 {
            return Some(CommentPosition::After);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_comments() {
        let source = r#"// First comment
fn main() {
    let x = 5; // inline comment
    // Another comment
    return x;
}"#;

        let comments = scan_comments(source);
        assert_eq!(comments.len(), 3);

        assert_eq!(comments[0].text, "// First comment");
        assert_eq!(comments[0].span.start, 0);
        assert_eq!(comments[0].span.end, 16);
        assert!(comments[0].is_line);

        assert_eq!(comments[1].text, "// inline comment");
        assert!(comments[1].is_line);

        assert_eq!(comments[2].text, "// Another comment");
        assert!(comments[2].is_line);
    }

    #[test]
    fn test_comment_position() {
        let source = "// before\nfn main() {} // end of line\n// after";

        // Node spans from "fn" to "}"
        let node_start = source.find("fn").unwrap();
        let node_end = source.find("}").unwrap() + 1;
        let node_span = SimpleSpan::<usize>::from(node_start..node_end);

        // Test before comment
        let before_span = SimpleSpan::<usize>::from(0..9);
        assert_eq!(
            determine_comment_position(before_span, node_span, source),
            Some(CommentPosition::Before)
        );

        // Test end-of-line comment
        let eol_start = source.find("// end").unwrap();
        let eol_span = SimpleSpan::<usize>::from(eol_start..eol_start + 14);
        assert_eq!(
            determine_comment_position(eol_span, node_span, source),
            Some(CommentPosition::EndOfLine)
        );

        // Test after comment
        let after_start = source.rfind("//").unwrap();
        let after_span = SimpleSpan::<usize>::from(after_start..source.len());
        assert_eq!(
            determine_comment_position(after_span, node_span, source),
            Some(CommentPosition::After)
        );
    }
}
