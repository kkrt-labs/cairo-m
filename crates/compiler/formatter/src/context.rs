use chumsky::span::SimpleSpan;

use crate::comment_preserver::CommentPreserver;
use crate::config::FormatterConfig;
use crate::trivia::{Comment, CommentBuckets};

pub struct FormatterCtx<'a> {
    pub cfg: &'a FormatterConfig,
    pub comments: CommentBuckets,
    pub comment_preserver: CommentPreserver,
    pub source: &'a str,
    /// Current line number being formatted
    pub current_line: usize,
}

impl<'a> FormatterCtx<'a> {
    pub fn new(cfg: &'a FormatterConfig, source: &'a str) -> Self {
        let comment_preserver = CommentPreserver::from_source(source);
        Self {
            cfg,
            comments: CommentBuckets::default(),
            comment_preserver,
            source,
            current_line: 0,
        }
    }

    pub fn get_leading_comments(&self, span: SimpleSpan<usize>) -> Option<&[Comment]> {
        self.comments.get_leading(span)
    }

    pub fn get_trailing_comments(&self, span: SimpleSpan<usize>) -> Option<&[Comment]> {
        self.comments.get_trailing(span)
    }

    pub fn set_comments(&mut self, comments: CommentBuckets) {
        self.comments = comments;
    }
}
