#!/usr/bin/env bash
set -euo pipefail

NEW_VERSION=$(sed "s/\./\\\./" .github/version)-$GITHUB_REF

REPLACE=$(printf '%s\n' "$NEW_REPLACE" | sed -e 's/[\/&]/\\&/g')

sed "s/{version}/$REPLACE/g" "./.github/templates/README.md" > README.md

sed "s/{version}/$REPLACE/g" "./.github/templates/Cargo.toml" > Cargo.toml
