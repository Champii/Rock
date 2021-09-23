#!/usr/bin/env bash
set -euo pipefail

# NEW_VERSION=$(cat .github/version)-$GITHUB_REF
NEW_VERSION=test-version

sed "s/{version}/$NEW_VERSION/g" "./.github/templates/README.md" > README.md

sed "s/{version}/$NEW_VERSION/g" "./.github/templates/Cargo.toml" > Cargo.toml
