# Cairo-M VS Code Extension - Complete Usage Guide

This guide covers how to install, configure, and use the Cairo-M Language Server
and VS Code extension for an optimal development experience.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Installation](#installation)
3. [Configuration](#configuration)

## Prerequisites

Before installing the Cairo-M VS Code extension, ensure you have:

- **VS Code** version 1.75.0 or higher
- **Rust** toolchain (for building the language server)
- **Node.js** and npm (for building the extension)
- **Git** (for cloning the repository)

### Installing Prerequisites

#### Rust Toolchain

```bash
# Install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add to PATH (if not done automatically)
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

#### Node.js

Download and install from [nodejs.org](https://nodejs.org/) or use a version
manager:

```bash
# Using nvm (Node Version Manager)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install node
nvm use node
```

## Installation

### Option 1: From Source (Recommended for Now)

#### Step 1: Clone the Repository

```bash
git clone https://github.com/yourusername/cairo-m.git
cd cairo-m

# Initialize submodules (required for Stwo)
git submodule update --init --recursive
```

#### Step 2: Build the Language Server

```bash
# Build the language server in release mode
cargo build --release -p cairo-m-ls

# Verify the binary was created
ls -la target/release/cairo-m-ls
```

The language server binary will be at `target/release/cairo-m-ls` (or
`cairo-m-ls.exe` on Windows).

#### Step 3: Build the VS Code Extension

```bash
# Navigate to the extension directory
cd vscode-cairo-m

# Install dependencies
npm install

# Compile TypeScript
npm run compile

# Package the extension
npm run package

# This creates a .vsix file
ls -la *.vsix
```

#### Step 4: Install the Extension in VS Code

1. Open VS Code
2. Go to Extensions (Ctrl+Shift+X / Cmd+Shift+X)
3. Click the "..." menu at the top of the Extensions panel
4. Select "Install from VSIX..."
5. Navigate to the `.vsix` file you created and select it
6. Restart VS Code

### Option 2: Manual Installation (Advanced Users)

If you want to install components separately:

1. **Language Server**: Copy the `cairo-m-ls` binary to a location in your PATH:

   ```bash
   # Linux/macOS
   sudo cp target/release/cairo-m-ls /usr/local/bin/

   # Or add to PATH
   export PATH=$PATH:/path/to/cairo-m/target/release
   ```

2. **VS Code Extension**: Install the VSIX as described above

## Configuration

### Extension Settings

Open VS Code settings (Ctrl+, / Cmd+,) and search for "Cairo-M" to find these
options:

#### Language Server Path

If the extension can't find the language server automatically, set the path
manually:

```json
{
  "cairo-m.languageServer.path": "/path/to/cairo-m-ls"
}
```

On Windows:

```json
{
  "cairo-m.languageServer.path": "C:\\path\\to\\cairo-m-ls.exe"
}
```

#### Trace Server Communication

For debugging, you can enable tracing:

```json
{
  "cairo-m.trace.server": "verbose"
}
```

Options: `"off"`, `"messages"`, `"verbose"`

### Workspace Settings

Create a `.vscode/settings.json` in your Cairo-M project:

```json
{
  "files.associations": {
    "*.cm": "cairo-m"
  },
  "editor.formatOnSave": false,
  "[cairo-m]": {
    "editor.semanticHighlighting.enabled": true,
    "editor.suggest.insertMode": "replace"
  }
}
```
