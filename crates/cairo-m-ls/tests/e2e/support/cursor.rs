use lsp_types::Position;

/// Represents a cursor position in the text
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub position: Position,
}

/// Collection of cursors extracted from test input
#[derive(Debug, Default, Clone)]
pub struct Cursors {
    pub carets: Vec<Cursor>,
    pub selections: Vec<(Position, Position)>,
}

impl Cursors {
    /// Assert that there is exactly one caret and return its position
    pub fn assert_single_caret(&self) -> Position {
        assert_eq!(
            self.carets.len(),
            1,
            "Expected exactly one caret, found {}",
            self.carets.len()
        );
        assert!(
            self.selections.is_empty(),
            "Expected no selections, found {}",
            self.selections.len()
        );
        self.carets[0].position
    }

    /// Assert that there is exactly one selection and return its range
    pub fn assert_single_selection(&self) -> (Position, Position) {
        assert_eq!(
            self.selections.len(),
            1,
            "Expected exactly one selection, found {}",
            self.selections.len()
        );
        assert!(
            self.carets.is_empty(),
            "Expected no carets, found {}",
            self.carets.len()
        );
        self.selections[0]
    }
}

/// Extract cursors from test input text
/// - `<caret>` marks cursor positions
/// - `<sel>...</sel>` marks selections
///
/// # Returns
/// * the cleaned text
/// * the extracted cursors (carets and selections)
pub fn extract_cursors(input: &str) -> (String, Cursors) {
    let mut cursors = Cursors::default();
    let mut output = String::new();
    let mut line = 0;
    let mut col = 0;
    let mut chars = input.chars().peekable();
    let mut selection_start: Option<Position> = None;

    // Track the column position in the output (without markers)
    let mut output_col = 0;
    let mut output_line = 0;

    while let Some(ch) = chars.next() {
        // Check for markers
        if ch == '<' {
            // Try to parse a marker
            let mut marker = String::from("<");
            let marker_start_line = line;
            let marker_start_col = col;

            // Position in the output where the marker would have been
            let cursor_line = output_line;
            let cursor_col = output_col;

            // Collect characters until we hit '>' or run out
            let mut found_marker = false;
            while let Some(&next_ch) = chars.peek() {
                chars.next();
                marker.push(next_ch);
                if next_ch == '\n' {
                    line += 1;
                    col = 0;
                } else {
                    col += 1;
                }

                if next_ch == '>' {
                    found_marker = true;
                    break;
                }
            }

            if found_marker {
                match marker.as_str() {
                    "<caret>" => {
                        // Record cursor position (where it would be in the clean output)
                        cursors.carets.push(Cursor {
                            position: Position {
                                line: cursor_line as u32,
                                character: cursor_col as u32,
                            },
                        });
                    }
                    "<sel>" => {
                        // Start of selection
                        selection_start = Some(Position {
                            line: cursor_line as u32,
                            character: cursor_col as u32,
                        });
                    }
                    "</sel>" => {
                        // End of selection
                        if let Some(start) = selection_start.take() {
                            let end = Position {
                                line: cursor_line as u32,
                                character: cursor_col as u32,
                            };
                            cursors.selections.push((start, end));
                        }
                    }
                    _ => {
                        // Not a recognized marker, add it to output
                        output.push_str(&marker);
                        // Update output position for non-marker content
                        for ch in marker.chars() {
                            if ch == '\n' {
                                output_line += 1;
                                output_col = 0;
                            } else {
                                output_col += 1;
                            }
                        }
                    }
                }
            } else {
                // Incomplete marker, restore position and add to output
                output.push_str(&marker);
                line = marker_start_line;
                col = marker_start_col + marker.len();
                // Update output position
                for ch in marker.chars() {
                    if ch == '\n' {
                        output_line += 1;
                        output_col = 0;
                    } else {
                        output_col += 1;
                    }
                }
            }
        } else {
            // Regular character
            output.push(ch);
            if ch == '\n' {
                line += 1;
                col = 0;
                output_line += 1;
                output_col = 0;
            } else {
                col += 1;
                output_col += 1;
            }
        }
    }

    (output, cursors)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_single_caret() {
        let input = "let x<caret> = 5;";
        let (output, cursors) = extract_cursors(input);

        assert_eq!(output, "let x = 5;");
        assert_eq!(cursors.carets.len(), 1);
        assert_eq!(cursors.selections.len(), 0);

        let pos = cursors.assert_single_caret();
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 5);
    }

    #[test]
    fn test_extract_multiple_carets() {
        let input = "let <caret>x = <caret>5;";
        let (output, cursors) = extract_cursors(input);

        assert_eq!(output, "let x = 5;");
        assert_eq!(cursors.carets.len(), 2);
        assert_eq!(cursors.carets[0].position.line, 0);
        assert_eq!(cursors.carets[0].position.character, 4);
        assert_eq!(cursors.carets[1].position.line, 0);
        assert_eq!(cursors.carets[1].position.character, 8);
    }

    #[test]
    fn test_extract_selection() {
        let input = "let <sel>x = 5</sel>;";
        let (output, cursors) = extract_cursors(input);

        assert_eq!(output, "let x = 5;");
        assert_eq!(cursors.selections.len(), 1);
        assert_eq!(cursors.carets.len(), 0);

        let (start, end) = cursors.assert_single_selection();
        assert_eq!(start.line, 0);
        assert_eq!(start.character, 4);
        assert_eq!(end.line, 0);
        assert_eq!(end.character, 9);
    }

    #[test]
    fn test_extract_multiline_caret() {
        let input = "let x = 5;\n<caret>let y = 10;";
        let (output, cursors) = extract_cursors(input);

        assert_eq!(output, "let x = 5;\nlet y = 10;");
        assert_eq!(cursors.carets.len(), 1);

        let pos = cursors.assert_single_caret();
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_extract_multiline_selection() {
        let input = "let <sel>x = 5;\nlet y</sel> = 10;";
        let (output, cursors) = extract_cursors(input);

        assert_eq!(output, "let x = 5;\nlet y = 10;");
        assert_eq!(cursors.selections.len(), 1);

        let (start, end) = cursors.assert_single_selection();
        assert_eq!(start.line, 0);
        assert_eq!(start.character, 4);
        assert_eq!(end.line, 1);
        assert_eq!(end.character, 5);
    }

    #[test]
    fn test_no_markers() {
        let input = "let x = 5;";
        let (output, cursors) = extract_cursors(input);

        assert_eq!(output, "let x = 5;");
        assert_eq!(cursors.carets.len(), 0);
        assert_eq!(cursors.selections.len(), 0);
    }

    #[test]
    fn test_incomplete_marker() {
        let input = "let x < 5;";
        let (output, cursors) = extract_cursors(input);

        assert_eq!(output, "let x < 5;");
        assert_eq!(cursors.carets.len(), 0);
        assert_eq!(cursors.selections.len(), 0);
    }

    #[test]
    fn test_unrecognized_marker() {
        let input = "let x <unknown> = 5;";
        let (output, cursors) = extract_cursors(input);

        assert_eq!(output, "let x <unknown> = 5;");
        assert_eq!(cursors.carets.len(), 0);
        assert_eq!(cursors.selections.len(), 0);
    }
}
