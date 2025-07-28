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

## Phase 3: Comment Support

### 3.1 Capture comments

- [ ] Decide on lexer extension vs second-pass
- [ ] Implement comment capture
- [ ] Create comment attachment logic

### 3.2 Update formatting rules

- [ ] Add comment emission to all rules
- [ ] Test comment preservation
- [ ] Add comment-specific tests

## Phase 4: Range Formatting

### 4.1 Implement range formatting

- [ ] Add format_range API
- [ ] Implement minimal diff computation
- [ ] Add LSP rangeFormatting handler

## Phase 5: Configuration & Polish

### 5.1 VSCode settings

- [ ] Add formatter settings to package.json
- [ ] Map VSCode settings to FormatterConfig
- [ ] Test configuration changes

### 5.2 Documentation

- [ ] Create README for formatter crate
- [ ] Add developer documentation
- [ ] Update main project docs
