#!/bin/bash

# Installation script for tree-sitter-rock
# This script will compile and install the tree-sitter parser for Neovim

set -e

echo "🪨 Installing tree-sitter-rock for Rock language syntax highlighting"
echo "======================================================================"
echo ""

# Check if we're in the right directory
if [ ! -f "grammar.js" ]; then
    echo "❌ Error: grammar.js not found. Please run this script from the tree-sitter-rock directory."
    exit 1
fi

# Check for node
if ! command -v node &> /dev/null; then
    echo "❌ Error: Node.js is not installed. Please install Node.js first."
    echo "   Visit: https://nodejs.org/"
    exit 1
fi

# Check for tree-sitter CLI
if ! command -v tree-sitter &> /dev/null; then
    echo "📦 Installing tree-sitter CLI..."
    npm install -g tree-sitter
else
    echo "✅ tree-sitter CLI found"
fi

# Install dependencies
echo ""
echo "📦 Installing npm dependencies..."
npm install

# Generate parser
echo ""
echo "🔨 Generating parser from grammar.js..."
npx tree-sitter generate

# Build parser
echo ""
echo "🔨 Compiling parser..."
npx tree-sitter build

# Test parser
echo ""
echo "🧪 Testing parser..."
npx tree-sitter test || echo "⚠️  Some tests failed, but parser was built"

echo ""
echo "======================================================================"
echo "✅ Installation complete!"
echo ""
echo "📝 Next steps:"
echo "   1. Copy the contents of 'neovim-setup.lua' to your Neovim config"
echo "   2. Update the path in neovim-setup.lua to point to:"
echo "      $(pwd)"
echo "   3. Restart Neovim and open any .rk file"
echo "   4. Run :TSInstall rock in Neovim (if using nvim-treesitter)"
echo ""
echo "📚 See INSTALL.md or QUICKSTART.md for detailed instructions"
echo ""
