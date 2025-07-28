# Cairo-M Formatter Implementation Plan

## Phase 1: MVP (No Comments, Full-File) ✅

### 1.1 Create formatter crate structure

- [x] Create `crates/formatter` directory
- [x] Create `Cargo.toml` with dependencies
- [x] Create source file structure
- [x] Add to workspace `Cargo.toml`
- [x] Add chumsky to workspace dependencies
- [x] Verify project builds (partial)

### 1.2 Implement Doc IR and renderer

- [x] Create `doc.rs` with Doc enum
- [x] Implement basic Doc constructors
- [x] Implement Wadler/Leijen pretty-printer
- [ ] Add unit tests for Doc rendering

### 1.3 Create formatter config and context

- [x] Create `config.rs` with FormatterConfig
- [x] Create `context.rs` with FormatterCtx
- [x] Add default configuration values

### 1.4 Implement basic formatting rules

- [x] Create Format trait in `lib.rs`
- [x] Implement rules for items (functions, structs, namespaces, const, use)
- [x] Implement rules for statements (let, const, if, while, for, etc.)
- [x] Implement rules for expressions (binary, unary, literals, calls, etc.)
- [x] Implement rules for types (named, pointer, tuple)
- [x] Implement rules for patterns (identifier, tuple)

### 1.5 Create public API

- [x] Implement `format_source_file` in `api.rs`
- [x] Implement `format_parsed_module`
- [x] Fix API to use correct parser types
- [ ] Add Salsa integration (optional for MVP)

### 1.6 Add snapshot tests

- [x] Set up insta for snapshot testing
- [x] Create test fixtures (simple function, struct, if, namespace, const, use)
- [x] Add idempotence tests
- [x] Verify all tests pass

## Phase 2: LSP Integration ✅

### 2.1 Add LSP capabilities

- [x] Update initialize() to advertise formatting
- [x] Implement textDocument/formatting handler
- [x] Implement textDocument/rangeFormatting handler
- [x] Add cairo-m-formatter dependency to LSP
- [x] Test build of LSP with formatter

### 2.2 LSP Integration Tests

- [x] Create LSP formatting tests
- [x] Test basic formatting
- [x] Test struct formatting
- [x] Test empty file handling

## Phase 3: Comment Support ✅

### 3.1 Capture comments ✅

- [x] Decide on lexer extension vs second-pass (chose second-pass)
- [x] Implement comment capture (scan_comments function)
- [x] Create comment attachment logic (CommentPreserver + comment_attachment)

### 3.2 Update formatting rules ✅

- [x] Add basic comment emission (file-level comments)
- [x] Test comment preservation
- [x] Add comment-specific tests
- [x] Full inline/end-of-line comment support using AST spans
- [x] Implement HasSpan trait for Spanned<T> types
- [x] Add CommentBuckets attachment system
- [x] Update statement and top-level item formatters

## Phase 4: Configuration & Polish

### 4.1 VSCode settings

- [ ] Add formatter settings to package.json
- [ ] Map VSCode settings to FormatterConfig
- [ ] Test configuration changes

### 4.2 Documentation

- [ ] Create README for formatter crate
- [ ] Add developer documentation
- [ ] Update main project docs
