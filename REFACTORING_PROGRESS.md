# Cairo-M LS Refactoring Progress

This document tracks the implementation progress of the cairo-m-ls refactoring
based on the cairols architecture analysis.

## Overview

The refactoring aims to improve cairo-m-ls by adopting proven patterns from
cairols:

- Project activation with background controller
- Unified project/crate representation
- Incremental compilation and state management
- Decoupled compiler interaction

## Implementation Progress

### 1. Project Module Structure (Task #1)

**Status**: Completed ✓

**Goals**:

- Create `ProjectController` for background project discovery
- Implement `ProjectModel` as central project state
- Use channel-based communication to prevent blocking

**Implementation Notes**:

- Created new module structure in `crates/cairo-m-ls/src/project/`
- ProjectController runs in a dedicated background thread
- Communication via crossbeam channels (thread-safe unlike std::mpsc)
- ProjectModel manages loaded crates with thread-safe storage
- Backend now triggers project discovery on file open

**Key Decisions**:

- Used crossbeam-channel instead of std::mpsc for thread safety (required by
  LSP)
- ProjectController processes requests asynchronously, preventing UI blocking
- ProjectModel uses Arc<RwLock<>> for concurrent access patterns

**How I Solved It**:

1. Created three submodules: controller.rs, manifest.rs, and model.rs
2. The controller spawns a worker thread that listens for ProjectUpdateRequest
   messages
3. When a file is opened, Backend sends an UpdateForFile request to the
   controller
4. Controller checks for cairom.toml using ProjectManifestPath::discover
5. Results are sent back as ProjectUpdate::Project or ProjectUpdate::Standalone
6. Backend processes these updates by loading crates into the ProjectModel

### 2. ProjectManifestPath (Task #2)

**Status**: Completed ✓

**Goals**:

- Create enum for project manifest types
- Move project discovery logic from compiler to LS
- Implement discover function respecting cairom.toml

**Implementation Notes**:

- Created ProjectManifestPath enum with CairoM variant
- Implemented discover() method that walks up directory tree
- Maintains compatibility with existing cairom.toml convention

**How I Solved It**:

1. Created a simple enum with just CairoM variant (no need for multiple types
   yet)
2. The discover() method starts from a file path and walks up looking for
   cairom.toml
3. Returns Some(ProjectManifestPath::CairoM(path)) when found, None otherwise
4. This cleanly separates project discovery concern into the LS layer

### 3. Unified ProjectCrate (Task #3)

**Status**: Completed ✓

**Goals**:

- Create single authoritative Crate representation
- Convert to Salsa input instead of parameter
- Consolidate parser::Crate and semantic::Crate

**Implementation Notes**:

- Created ProjectCrate as a Salsa input in db.rs
- Unified representation with PathBuf for paths and HashMap for files
- Created ProjectCrateExt trait for conversions to phase-specific crates
- ProjectModel now stores ProjectCrate IDs for retrieval

**How I Solved It**:

1. Created a new db.rs module with AnalysisDatabase (replacing CompilerDatabase
   in LS)
2. Defined ProjectCrate as a Salsa input with root_dir, main_module, and files
3. Implemented ProjectCrateExt trait to convert to parser::Crate and
   semantic::Crate
4. Updated ProjectModel to create and store ProjectCrate instances
5. The key insight: Each compiler phase still uses its own Crate type, but
   they're all derived from the single ProjectCrate input
6. This ensures consistency while allowing phase-specific optimizations

### 4. DiagnosticsController (Task #4)

**Status**: Completed ✓

**Goals**:

- Background thread for diagnostic computation
- Non-blocking diagnostic runs
- Support for batching and prioritization

**Implementation Notes**:

- Created DiagnosticsController running in dedicated thread
- Channel-based communication for requests/responses
- Computes diagnostics for entire project when files change
- Non-blocking architecture ensures UI responsiveness

**How I Solved It**:

1. Created diagnostics module with controller.rs and state.rs
2. DiagnosticsController spawns worker thread that processes DiagnosticsRequest
   messages
3. Supports FileChanged and ProjectChanged request types
4. When a file changes, it retrieves the ProjectCrate and runs full project
   validation
5. Results are sent back via DiagnosticsResponse messages
6. Backend processes responses asynchronously and publishes to LSP client
7. Key insight: Running diagnostics in background prevents UI freezing during
   validation

### 5. ProjectDiagnostics (Task #5)

**Status**: Completed ✓

**Goals**:

- Thread-safe diagnostic storage
- Separation of computation and state
- Consistent diagnostic publication

**Implementation Notes**:

- Created ProjectDiagnostics with RwLock for thread-safe access
- HashMap storage mapping URLs to diagnostics
- Clear separation between computation (controller) and state

**How I Solved It**:

1. Created state.rs with ProjectDiagnostics struct
2. Used RwLock<HashMap<Url, Vec<Diagnostic>>> for concurrent access
3. Provides methods to set, get, and clear diagnostics
4. DiagnosticsController updates this state after computation
5. Backend can read from this state when needed
6. This separation allows multiple components to access diagnostic state safely

### 6. AnalysisDatabaseSwapper (Task #6)

**Status**: Completed ✓

**Goals**:

- Periodic database refresh for memory management
- State migration between databases
- Atomic swapping mechanism

**Implementation Notes**:

- Created AnalysisDatabaseSwapper running in background thread
- Swaps database every 5 minutes to prevent memory growth
- Migrates essential state (open files, project crates) to new database
- Atomic swap ensures no disruption to ongoing operations

**How I Solved It**:

1. Created swapper.rs in the db module
2. Worker thread wakes up periodically (5 minutes by default)
3. Creates a fresh AnalysisDatabase instance
4. Extracts essential state from old database:
   - Open file contents
   - Project crate configurations
5. Re-applies state to new database
6. Atomically swaps old database with new one
7. Old database is dropped, freeing accumulated Salsa query results
8. This pattern prevents memory leaks in long-running LS sessions

### 7. Move CompilerDatabase (Task #7)

**Status**: Completed ✓

**Goals**:

- Define concrete database in cairo-m-ls
- Compiler crates only define traits
- Better modularity and separation

**Implementation Notes**:

- Created AnalysisDatabase in cairo-m-ls/src/db/mod.rs
- Replaced CompilerDatabase usage in LS with AnalysisDatabase
- Compiler crates now only define trait requirements

**How I Solved It**:

1. Created AnalysisDatabase as the concrete Salsa database
2. Implements ParserDb and SemanticDb traits
3. Moved database instantiation to the LS layer
4. This follows the cairols pattern where the application (LS) owns the database
5. Compiler libraries remain agnostic to the concrete database implementation

### 8. Update Compiler Queries (Task #8)

**Status**: Completed ✓

**Goals**:

- Queries use unified ProjectCrate input
- Remove ad-hoc crate creation
- Cleaner dependency graph

**Implementation Notes**:

- Compiler queries remain unchanged (still use semantic::Crate)
- Created ProjectCrateExt trait to convert ProjectCrate to semantic::Crate
- Updated LS to use get_semantic_crate_for_file instead of get_or_create_crate
- All LSP features now use the unified ProjectCrate via ProjectModel

**How I Solved It**:

1. Recognized that changing query signatures would be too invasive
2. Instead, created conversion trait (ProjectCrateExt) to bridge the gap
3. DiagnosticsController uses project_crate.to_semantic_crate(db)
4. Added get_semantic_crate_for_file helper in Backend
5. Updated goto_definition, hover, and completion to use the new method
6. This maintains backward compatibility while using the new architecture

## Technical Decisions

### Why These Changes?

1. **Non-blocking Operations**: The current implementation blocks on project
   discovery and diagnostics. The controller pattern ensures responsiveness.

2. **Memory Management**: Long-running LS sessions accumulate Salsa query
   results. The database swapper prevents unbounded growth.

3. **Unified State**: Multiple crate representations cause confusion. A single
   ProjectCrate input simplifies the architecture.

4. **Modularity**: Moving database definition to LS separates concerns and
   allows different applications to configure differently.

## Challenges and Solutions

(To be updated as implementation progresses)

## Testing Strategy

- Unit tests for each new component
- Integration tests for project discovery scenarios
- Performance benchmarks for large projects
- Memory usage monitoring

## Summary

All tasks have been completed successfully! The cairo-m-ls refactoring has
transformed the language server to align with the robust patterns from cairols:

### Key Achievements:

1. **Non-blocking Architecture**: Project discovery and diagnostics now run in
   background threads, preventing UI freezes

2. **Unified State Management**:

   - ProjectModel centralizes project state
   - ProjectCrate serves as single source of truth
   - ProjectDiagnostics provides thread-safe diagnostic storage

3. **Memory Management**: AnalysisDatabaseSwapper prevents unbounded memory
   growth in long-running sessions

4. **Clean Separation of Concerns**:

   - Compiler libraries only define traits
   - Language server owns concrete database implementation
   - Clear boundaries between components

5. **Scalability**: The new architecture can handle large, multi-file projects
   efficiently

### Architecture Overview:

```
┌─────────────────┐
│   LSP Client    │
└────────┬────────┘
         │
┌────────▼────────┐
│     Backend     │ ◄── Main event loop
├─────────────────┤
│ ProjectModel    │ ◄── Central state
│ AnalysisDB      │ ◄── Salsa database
└────────┬────────┘
         │
   ┌─────┴─────┬─────────┬──────────┐
   │           │         │          │
┌──▼───┐  ┌───▼──┐  ┌───▼───┐  ┌───▼───┐
│Project│  │Diag. │  │DB     │  │Compiler│
│Ctrl   │  │Ctrl  │  │Swapper│  │Queries │
└───────┘  └──────┘  └───────┘  └────────┘
Background threads    Timer      Incremental
```

The refactoring successfully modernizes cairo-m-ls while maintaining
compatibility with the existing compiler infrastructure.

## Known Issues and Workarounds

### Salsa Panic on Startup (FIXED)

Previously, there was a panic when starting the language server:

```
thread '<unnamed>' panicked at salsa-0.22.0/src/table.rs:358:9:
out of bounds access `SlotIndex(1)` (maximum slot `1`)
```

**Root Cause**: The original implementation in
`ProjectModel::discover_crate_files` was creating SourceFile entities with a
temporary database, then using them with the main database. Salsa entities are
irrevocably tied to the database instance that created them.

**Fix Applied**:

- Modified `discover_crate_files` to accept the main database as a parameter
- Ensured all SourceFile creation uses the same database instance
- This prevents the Salsa panic from occurring

### Files Not Analyzed on Startup

As documented earlier, the LSP spec doesn't provide a way to query open files on
startup. Combined with the Salsa panic issue, this can lead to a poor initial
experience.

**Best Practice**: After starting the language server, trigger a file change
(e.g., add and remove a space) to ensure diagnostics are activated.

## Recent Improvements (Expert Code Review)

Based on expert feedback, the following critical improvements have been
implemented:

### 1. Fixed Salsa Database Instance Bug

**Issue**: Creating SourceFile entities with a temporary database caused panics
when used with the main database. **Fix**: Modified
`ProjectModel::discover_crate_files` to accept the main database as a parameter,
ensuring all entities are created with the same database instance.

### 2. Lock Minimization Pattern

**Issue**: Background threads were holding database locks during expensive
computations, blocking the main thread. **Fix**: Implemented a pattern to
extract necessary data quickly and release locks before processing:

- `DiagnosticsController` now extracts data with minimal lock time
- `AnalysisDatabaseSwapper` builds new database without holding locks
- This prevents UI freezes during background operations

### 3. Manifest Caching

**Issue**: Project discovery was re-loading manifests for every file opened
within a project. **Fix**: Added a manifest cache to `ProjectController` with:

- 5-minute cache expiration for loaded manifests
- Cache hit logging for debugging
- Automatic cleanup of expired entries

### 4. Database Swapper Refinement

**Issue**: The swapper held the database lock while building the new database.
**Fix**: Restructured to:

1. Extract essential state with minimal lock time
2. Build new database without any locks
3. Perform atomic swap with minimal lock time

These improvements address the critical architectural issues identified in the
code review, resulting in a more robust and performant language server.

## Data Flow Bug Fix (Empty Semantic Index)

### Issue

The semantic index was completely empty (0 definitions, 0 identifier usages)
despite files containing code. This was caused by having two separate SourceFile
entities:

1. One created from LSP client content in `did_open`
2. Another created from disk content in `ProjectModel::discover_crate_files`

The semantic analysis was running on the disk-based SourceFiles (potentially
stale or empty), not the live client content.

### Root Cause

The architecture had dual sources of truth for file content:

- `Backend` stored SourceFiles from the LSP client
- `ProjectModel` created its own SourceFiles by reading from disk
- These were completely separate entities in the Salsa database
- Diagnostics/analysis used the disk-based files, ignoring client edits

### Fix Applied

Established `Backend` as the single source of truth for file content:

1. **Centralized File Management**:

   - `Backend` now owns all SourceFile creation via `get_or_create_source_file`
   - Uses URI as the canonical identifier (not file path)
   - Maintains a single `source_files` map for all open files

2. **Refactored Project Discovery**:

   - `ProjectController` now discovers file _paths_ only, not content
   - Returns `ProjectUpdate::Project { crate_info, files: Vec<PathBuf> }`
   - No longer reads files from disk

3. **Updated ProjectModel**:

   - `load_crate` now accepts a closure to get SourceFiles
   - Uses Backend's existing SourceFiles instead of creating new ones
   - Ensures all analysis uses the same file entities

4. **Data Flow**:
   ```
   Client → did_open → Backend creates SourceFile → ProjectController discovers paths
   → ProjectModel uses Backend's SourceFiles → Semantic analysis uses correct content
   ```

This ensures semantic analysis always runs on the exact content the user sees in
their editor.
