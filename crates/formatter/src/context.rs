use crate::config::FormatterConfig;
use crate::trivia::CommentBuckets;

pub struct FormatterCtx<'a> {
    pub cfg: &'a FormatterConfig,
    pub comments: CommentBuckets,
    pub source: &'a str,
}

impl<'a> FormatterCtx<'a> {
    pub fn new(cfg: &'a FormatterConfig, source: &'a str) -> Self {
        Self {
            cfg,
            comments: CommentBuckets::default(),
            source,
        }
    }
}
