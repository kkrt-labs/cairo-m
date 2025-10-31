use std::collections::BTreeMap;

use crate::trivia::{Comment, scan_comments};

/// Preserves comments by tracking their line positions in the original source
#[derive(Debug)]
pub struct CommentPreserver {
    /// Maps line numbers to comments that appear at the end of that line
    end_of_line_comments: BTreeMap<usize, Comment>,
    /// Maps line numbers to comments that appear on their own line
    standalone_comments: BTreeMap<usize, Vec<Comment>>,
}

impl CommentPreserver {
    pub fn from_source(source: &str) -> Self {
        let comments = scan_comments(source);
        let mut end_of_line_comments = BTreeMap::new();
        let mut standalone_comments = BTreeMap::new();

        for comment in comments {
            let line_number = source[..comment.span.start]
                .chars()
                .filter(|&c| c == '\n')
                .count();

            // Check if this comment is on its own line
            let line_start = if line_number == 0 {
                0
            } else {
                source[..comment.span.start]
                    .rfind('\n')
                    .map(|pos| pos + 1)
                    .unwrap_or(0)
            };

            let before_comment = &source[line_start..comment.span.start];
            let is_standalone = before_comment.trim().is_empty();

            if is_standalone {
                standalone_comments
                    .entry(line_number)
                    .or_insert_with(Vec::new)
                    .push(comment);
            } else {
                end_of_line_comments.insert(line_number, comment);
            }
        }

        Self {
            end_of_line_comments,
            standalone_comments,
        }
    }

    /// Get standalone comments that should appear before the given line
    pub fn get_standalone_comments_before(&self, line: usize) -> Option<&[Comment]> {
        self.standalone_comments.get(&line).map(|v| v.as_slice())
    }

    /// Get end-of-line comment for the given line
    pub fn get_end_of_line_comment(&self, line: usize) -> Option<&Comment> {
        self.end_of_line_comments.get(&line)
    }

    /// Calculate line number for a given byte offset
    pub fn line_number_for_offset(source: &str, offset: usize) -> usize {
        source[..offset.min(source.len())]
            .chars()
            .filter(|&c| c == '\n')
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_preserver() {
        let source = r#"// File header comment
// Second line of header

fn main() {
    let x = 5; // inline comment
    // standalone comment
    return x;
}"#;

        let preserver = CommentPreserver::from_source(source);

        // Check standalone comments - they're on different lines
        assert!(preserver.get_standalone_comments_before(0).is_some());
        assert_eq!(
            preserver.get_standalone_comments_before(0).unwrap().len(),
            1
        );

        assert!(preserver.get_standalone_comments_before(1).is_some());
        assert_eq!(
            preserver.get_standalone_comments_before(1).unwrap().len(),
            1
        );

        // Check end-of-line comment
        assert!(preserver.get_end_of_line_comment(4).is_some());
        let eol = preserver.get_end_of_line_comment(4).unwrap();
        assert_eq!(eol.text, "// inline comment");

        // Check another standalone comment
        assert!(preserver.get_standalone_comments_before(5).is_some());
        assert_eq!(
            preserver.get_standalone_comments_before(5).unwrap().len(),
            1
        );
    }
}
