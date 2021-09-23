#!/usr/bin/env bash
set -euo pipefail

cp ./.github/templates/README.md .
sed "s/{version}/${NEW_VERSION}/g" README.md

cp ./.github/templates/Cargo.toml .
sed "s/{version}/${NEW_VERSION}/g" Cargo.toml
