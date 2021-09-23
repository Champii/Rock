#!/usr/bin/env bash
set -euo pipefail

BRANCH=$(basename $GITHUB_REF)
NEW_VERSION=$(cat .github/version)-$BRANCH

REPLACE=$(printf '%s\n' "$NEW_VERSION" | sed -e 's/[\/&]/\\&/g')

sed "s/{version}/$REPLACE/g" "./.github/templates/README.md" > README.md
sed -i "s/{branch}/$BRANCH/g" "README.md"

REPLACE="${REPLACE:1}"

sed "s/{version}/$REPLACE/g" "./.github/templates/Cargo.toml" > Cargo.toml
