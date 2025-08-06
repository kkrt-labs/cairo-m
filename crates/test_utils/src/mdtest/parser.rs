use crate::mdtest::config::{Location, MdTestConfig, TestMetadata};
use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Failed to read file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Failed to parse TOML config: {0}")]
    TomlError(#[from] toml::de::Error),
    #[error("Invalid test annotation at line {line}: {message}")]
    InvalidAnnotation { line: usize, message: String },
}

#[derive(Debug, Clone)]
pub struct MdTest {
    pub name: String,
    pub cairo_source: String,
    pub rust_source: Option<String>,
    pub files: HashMap<String, String>,
    pub metadata: TestMetadata,
    pub location: Location,
    pub config: Option<MdTestConfig>,
}

pub fn extract_tests(markdown_path: &Path) -> Result<Vec<MdTest>, ParseError> {
    let content = std::fs::read_to_string(markdown_path)?;
    let mut tests = Vec::new();

    // Track section headings
    let mut current_h1 = String::new();
    let mut current_h2 = String::new();

    // Track current test components
    let mut current_config: Option<MdTestConfig> = None;
    let mut pending_cairo: Option<(String, TestMetadata, Location)> = None;
    let mut pending_rust: Option<String> = None;

    let mut in_heading = false;
    let mut heading_level = 0;
    let mut in_code_block = false;
    let mut code_block_lang = String::new();
    let mut code_block_content = String::new();
    let mut line_number = 1;

    let parser = Parser::new(&content);

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. }) => {
                // First, save any pending test before starting a new section
                if let Some((cairo_source, metadata, location)) = pending_cairo.take() {
                    let test_name = format_test_name(&current_h1, &current_h2);
                    tests.push(MdTest {
                        name: test_name,
                        cairo_source,
                        rust_source: pending_rust.take(),
                        files: HashMap::new(),
                        metadata,
                        location,
                        config: current_config.clone(),
                    });
                }

                in_heading = true;
                heading_level = level as usize;
            }
            Event::End(TagEnd::Heading(_)) => {
                in_heading = false;
            }
            Event::Text(ref text) if in_heading => match heading_level {
                1 => {
                    current_h1 = text.to_string();
                    current_h2.clear();
                }
                2 => {
                    current_h2 = text.to_string();
                }
                _ => {}
            },
            Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref lang))) => {
                in_code_block = true;
                code_block_lang = lang.to_string();
                code_block_content.clear();
            }
            Event::End(TagEnd::CodeBlock) => {
                // Process the completed code block
                match code_block_lang.as_str() {
                    "cairo-m" => {
                        // Save any previous test first
                        if let Some((cairo_source, metadata, location)) = pending_cairo.take() {
                            let test_name = format_test_name(&current_h1, &current_h2);
                            tests.push(MdTest {
                                name: test_name,
                                cairo_source,
                                rust_source: pending_rust.take(),
                                files: HashMap::new(),
                                metadata,
                                location,
                                config: current_config.clone(),
                            });
                        }

                        // Validate we have at least a level 1 heading
                        if current_h1.is_empty() {
                            eprintln!(
                                "Warning: Cairo-M code block at line {} appears before any heading in {}",
                                line_number,
                                markdown_path.display()
                            );
                        }

                        // Parse new Cairo-M code
                        let (source, metadata) =
                            parse_annotations(&code_block_content, line_number)?;
                        let location = Location::new(
                            markdown_path.to_string_lossy().to_string(),
                            line_number,
                            0,
                        );
                        pending_cairo = Some((source, metadata, location));
                    }
                    "rust" => {
                        // Store Rust code for the current pending Cairo test
                        pending_rust = Some(code_block_content.clone());
                    }
                    "toml" => {
                        current_config = Some(toml::from_str(&code_block_content)?);
                    }
                    _ => {}
                }

                in_code_block = false;
                code_block_content.clear();
            }
            Event::Text(ref text) if in_code_block => {
                code_block_content.push_str(text);
            }
            Event::Code(ref code) if in_code_block => {
                code_block_content.push_str(code);
            }
            _ => {}
        }

        // Approximate line counting
        if let Event::Text(ref text) = event {
            line_number += text.chars().filter(|&c| c == '\n').count();
        }
    }

    // Save any final pending test
    if let Some((cairo_source, metadata, location)) = pending_cairo {
        let test_name = format_test_name(&current_h1, &current_h2);
        tests.push(MdTest {
            name: test_name,
            cairo_source,
            rust_source: pending_rust,
            files: HashMap::new(),
            metadata,
            location,
            config: current_config,
        });
    }

    Ok(tests)
}

fn format_test_name(h1: &str, h2: &str) -> String {
    match (h1.is_empty(), h2.is_empty()) {
        (true, _) => {
            // Code block appears before any heading - generate a warning name
            "Orphaned Test (no heading)".to_string()
        }
        (false, true) => h1.to_string(),
        (false, false) => format!("{} - {}", h1, h2),
    }
}

fn parse_annotations(
    code: &str,
    _line_number: usize,
) -> Result<(String, TestMetadata), ParseError> {
    let mut metadata = TestMetadata::default();
    let mut source_lines = Vec::new();

    for line in code.lines() {
        if line.trim_start().starts_with("//!") {
            let annotation = line.trim_start().trim_start_matches("//!").trim();

            if let Some(expected) = annotation.strip_prefix("expected:") {
                metadata.expected_output = Some(expected.trim().to_string());
            } else if let Some(error) = annotation.strip_prefix("error:") {
                metadata.expected_error = Some(error.trim().trim_matches('"').to_string());
            } else if let Some(rust_equiv) = annotation.strip_prefix("rust-equiv:") {
                metadata.rust_equiv = Some(rust_equiv.trim().to_string());
            } else if let Some(tags) = annotation.strip_prefix("tags:") {
                let tags_str = tags.trim().trim_start_matches('[').trim_end_matches(']');
                metadata.tags = tags_str.split(',').map(|s| s.trim().to_string()).collect();
            } else if let Some(ignore) = annotation.strip_prefix("ignore:") {
                metadata.ignore = Some(ignore.trim().to_string());
            }
        } else {
            source_lines.push(line);
        }
    }

    Ok((source_lines.join("\n"), metadata))
}
