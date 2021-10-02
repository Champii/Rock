#!/usr/bin/env bash
set -euo pipefail

BRANCH=$(basename $GITHUB_REF)
NEW_VERSION=$(cat .github/version)-$BRANCH
REPLACE=$(printf '%s\n' "$NEW_VERSION" | sed -e 's/[\/&]/\\&/g')

BRANCH_SAFE=$(printf '%s' "$BRANCH" | sed -e 's/_/-/g')
NEW_VERSION_SAFE=$(cat .github/version)-$BRANCH_SAFE
REPLACE_SAFE=$(printf '%s\n' "$NEW_VERSION_SAFE" | sed -e 's/[\/&]/\\&/g')

sed "s/{version}/$REPLACE_SAFE/g" "./.github/templates/README.md" > README.md
sed -i "s/{branch}/$BRANCH/g" "README.md"

REPLACE_SAFE="${REPLACE_SAFE:1}"

sed "s/{version}/$REPLACE_SAFE/g" "./.github/templates/Cargo.toml" > Cargo.toml
