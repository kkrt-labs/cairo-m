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
