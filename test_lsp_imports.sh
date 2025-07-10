#!/bin/bash

# Test script to verify LSP import functionality

echo "Testing Cairo-M LSP import features..."

# 1. Test syntax highlighting
echo ""
echo "1. Checking VSCode syntax highlighting file..."
if grep -q '"use"' vscode-cairo-m/syntaxes/cairo-m.tmLanguage.json; then
    echo "✓ 'use' keyword is in syntax highlighting"
else
    echo "✗ 'use' keyword missing from syntax highlighting"
fi

if grep -q 'entity.name.module.cairo-m' vscode-cairo-m/syntaxes/cairo-m.tmLanguage.json; then
    echo "✓ Module name highlighting pattern exists"
else
    echo "✗ Module name highlighting pattern missing"
fi

# 2. Build the language server
echo ""
echo "2. Building language server..."
if cargo build -p cairo-m-ls --quiet; then
    echo "✓ Language server builds successfully"
else
    echo "✗ Language server build failed"
    exit 1
fi

# 3. Test compilation with imports
echo ""
echo "3. Testing compilation with imports..."
if cargo run -p cairo-m-compiler -- --input cairo-m-project/src 2>/dev/null; then
    echo "✓ Compilation with imports works"
else
    echo "✗ Compilation with imports failed"
fi

echo ""
echo "Test complete!"
echo ""
echo "To test go-to-definition manually:"
echo "1. Open VSCode with the Cairo-M extension"
echo "2. Open cairo-m-project/src/main.cm"
echo "3. Ctrl/Cmd+Click on 'math' in the import line"
echo "4. Ctrl/Cmd+Click on 'add' or 'sub' function calls"