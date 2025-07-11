Here is a well-defined issue ticket based on your request, ready for a
development team to pick up.

---

### Title: Improve LSP Project Discovery and Context Management

- **Labels:** `lsp`, `bug`, `enhancement`, `project-system`
- **Priority:** High

### What

The language server's project handling logic needs to be refined to correctly
handle standalone files and manage multiple project contexts simultaneously.
This involves two key improvements:

1.  Treating standalone `.cm` files (not part of a `cairom.toml` project) as
    single-file projects.
2.  Ensuring that analysis and diagnostic updates for one project do not
    interfere with others, especially when switching between files from
    different projects.

### Why

Currently, the LSP exhibits two problematic behaviors that lead to a confusing
and unreliable developer experience:

1.  **Incorrect Scoping for Standalone Files:** When a `.cm` file that is not
    part of a `cairom.toml` project is opened (e.g., a test case), the LSP
    incorrectly treats its entire parent directory as a single project. This
    causes it to parse and analyze all sibling files together, leading to
    incorrect cross-file diagnostics and slow performance in directories with
    many files.

2.  **Project Context Thrashing:** When a user switches between files from
    different projects (e.g., a real project and a test case), the LSP can get
    confused about the "active" project. It may run analysis for the newly
    focused project but fail to update or clear diagnostics for files from the
    previously focused one. This leaves stale diagnostics in the editor and
    prevents new diagnostics from appearing for the file being edited.

Resolving these issues is critical for providing a stable, accurate, and
responsive language server.

### How

The solution involves targeted changes within the language server crate
(`cairo-m-ls`) to better manage project context without altering the core
compiler's project discovery logic.

#### Part 1: Implement Single-File Project Handling

In `crates/cairo-m-ls/src/main.rs`, modify the `Backend::get_or_create_project`
method.

1.  After `find_project_root` determines a `project_root`, check for the
    existence of `cairom.toml` within that root.

    ```rust
    let is_real_project = project_root.join("cairom.toml").exists();
    ```

2.  If `cairom.toml` **is present**, the existing logic of calling
    `discover_project_files` and creating a multi-file project is correct and
    should be kept.

3.  If `cairom.toml` **is absent**, this indicates a standalone file. Instead of
    discovering all files in the directory, create a `Project` that contains
    _only_ the single `SourceFile` associated with the `file_uri` that triggered
    the call.

    - The `modules` `HashMap` should contain just one entry.
    - The `entry_point` for this single-file project will be the module name of
      the file itself.

This change isolates the behavior to the LSP, where the concept of a "temporary,
single-file project" makes sense, and keeps the core `project_discovery` crate
focused on `cairom.toml`-based projects.

> Note You will evaluate whether this should be a behavior isolated to the LSP
> or if it should be a behavior of the core compiler. I tend to believe that the
> core compiler should be able to handle this.

#### Part 2: Isolate Diagnostic Updates to the Correct Project

In `crates/cairo-m-ls/src/main.rs`, update the `Backend::run_diagnostics` method
to be strictly project-aware.

1.  After running `project_validate_semantics` and getting `diagnostics_by_file`
    for the current `project`, get a list of all file URIs that belong to that
    specific `project`.

    ```rust
    // In Backend::run_diagnostics, after project_validate_semantics
    let project_file_uris: std::collections::HashSet<Url> = self.safe_db_access(|db| {
        project.modules(db).values()
            .filter_map(|sf| Url::from_file_path(sf.file_path(db)).ok())
            .collect()
    }).unwrap_or_default();
    ```

2.  Iterate through `diagnostics_by_file` and publish new diagnostics as before.

3.  Finally, iterate through the `project_file_uris` set. For any URI in this
    set that does _not_ have an entry in `diagnostics_by_file`, it means all its
    previous errors have been fixed. Publish an empty diagnostic list for it to
    clear the editor's UI.

    ```rust
    // After publishing new diagnostics...
    for uri in project_file_uris {
        let path_str = uri.to_file_path().unwrap().display().to_string();
        if !diagnostics_by_file.contains_key(&path_str) {
            // This file is in the project but now has no diagnostics. Clear any old ones.
            self.client
                .publish_diagnostics(uri.clone(), vec![], None)
                .await;
        }
    }
    ```

This change ensures that when analyzing `Project A`, we only publish or clear
diagnostics for files within `Project A`. Diagnostics for any other open files
from `Project B` will remain untouched until an event triggers an analysis of
`Project B`.
