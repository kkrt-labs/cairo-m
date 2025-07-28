/// Pretty-print intermediate representation
#[derive(Debug, Clone, PartialEq)]
pub enum Doc {
    /// Empty document
    Nil,
    /// Plain text
    Text(String),
    /// Hard line break
    Line,
    /// Soft line break (space or newline)
    SoftLine,
    /// Group - tries to fit on one line
    Group(Box<Doc>),
    /// Indented block
    Indent(u32, Box<Doc>),
    /// Concatenation of multiple docs
    Concat(Vec<Doc>),
}

impl Doc {
    pub const fn nil() -> Self {
        Self::Nil
    }

    pub fn text<S: Into<String>>(s: S) -> Self {
        Self::Text(s.into())
    }

    pub const fn line() -> Self {
        Self::Line
    }

    pub const fn softline() -> Self {
        Self::SoftLine
    }

    pub fn group(doc: Self) -> Self {
        Self::Group(Box::new(doc))
    }

    pub fn indent(width: u32, doc: Self) -> Self {
        Self::Indent(width, Box::new(doc))
    }

    pub const fn concat(docs: Vec<Self>) -> Self {
        Self::Concat(docs)
    }

    /// Join documents with a separator
    pub fn join(sep: Self, docs: Vec<Self>) -> Self {
        if docs.is_empty() {
            return Self::Nil;
        }

        let mut result = vec![];
        for (i, doc) in docs.into_iter().enumerate() {
            if i > 0 {
                result.push(sep.clone());
            }
            result.push(doc);
        }
        Self::Concat(result)
    }

    /// Create a comment document
    pub fn comment(text: &str) -> Self {
        Self::text(text)
    }

    /// Append an end-of-line comment to a document
    pub fn with_eol_comment(self, comment: Option<&str>) -> Self {
        match comment {
            Some(text) => Self::concat(vec![self, Self::text(" "), Self::comment(text)]),
            None => self,
        }
    }

    /// Create a document with leading comments
    pub fn with_leading_comments<I>(comments: I, doc: Self) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut parts = Vec::new();
        for comment in comments {
            parts.push(Self::comment(&comment));
            parts.push(Self::line());
        }
        parts.push(doc);
        Self::concat(parts)
    }

    /// Create a document with trailing comments
    pub fn with_trailing_comments<I>(doc: Self, comments: I) -> Self
    where
        I: IntoIterator<Item = String>,
    {
        let mut parts = vec![doc];
        for comment in comments {
            parts.push(Self::text(" "));
            parts.push(Self::comment(&comment));
        }
        Self::concat(parts)
    }

    /// Render the document to a string with a given max width
    pub fn render(&self, max_width: usize) -> String {
        let mut renderer = Renderer::new(max_width);
        renderer.render_doc(self, Mode::Flat);
        renderer.output
    }
}

#[derive(Clone, Copy, Debug)]
enum Mode {
    Flat,
    Break,
}

struct Renderer {
    output: String,
    pos: usize,
    max_width: usize,
}

impl Renderer {
    const fn new(max_width: usize) -> Self {
        Self {
            output: String::new(),
            pos: 0,
            max_width,
        }
    }

    fn render_doc(&mut self, doc: &Doc, mode: Mode) {
        match doc {
            Doc::Nil => {}
            Doc::Text(s) => {
                self.output.push_str(s);
                self.pos += s.len();
            }
            Doc::Line => {
                self.output.push('\n');
                self.pos = 0;
            }
            Doc::SoftLine => match mode {
                Mode::Flat => {
                    self.output.push(' ');
                    self.pos += 1;
                }
                Mode::Break => {
                    self.output.push('\n');
                    self.pos = 0;
                }
            },
            Doc::Group(inner) => {
                let fits = self.fits(inner, self.max_width.saturating_sub(self.pos));
                let inner_mode = if fits { Mode::Flat } else { Mode::Break };
                self.render_doc(inner, inner_mode);
            }
            Doc::Indent(width, inner) => {
                // Render inner content to a separate string first
                let mut inner_renderer = Self::new(self.max_width);
                inner_renderer.render_doc(inner, mode);

                // Apply indentation to the rendered content
                let indent_str = " ".repeat(*width as usize);
                let mut at_line_start = self.pos == 0;

                for ch in inner_renderer.output.chars() {
                    if at_line_start && !ch.is_whitespace() {
                        self.output.push_str(&indent_str);
                        self.pos += *width as usize;
                        at_line_start = false;
                    }

                    self.output.push(ch);
                    if ch == '\n' {
                        self.pos = 0;
                        at_line_start = true;
                    } else {
                        self.pos += 1;
                    }
                }
            }
            Doc::Concat(docs) => {
                for doc in docs {
                    self.render_doc(doc, mode);
                }
            }
        }
    }

    fn fits(&self, doc: &Doc, width: usize) -> bool {
        Self::measure(doc, width, Mode::Flat).is_some()
    }

    fn measure(doc: &Doc, mut width: usize, mode: Mode) -> Option<usize> {
        match doc {
            Doc::Nil => Some(width),
            Doc::Text(s) => {
                if s.len() > width {
                    None
                } else {
                    Some(width - s.len())
                }
            }
            Doc::Line => None, // Hard breaks don't fit in flat mode
            Doc::SoftLine => match mode {
                Mode::Flat => width.checked_sub(1),
                Mode::Break => None,
            },
            Doc::Group(inner) => Self::measure(inner, width, Mode::Flat),
            Doc::Indent(_, inner) => Self::measure(inner, width, mode),
            Doc::Concat(docs) => {
                for doc in docs {
                    width = Self::measure(doc, width, mode)?;
                }
                Some(width)
            }
        }
    }
}
