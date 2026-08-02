#!/bin/bash

set -e

# Make the script run in its own directory instead of the caller's directory.
cd "$(cd -P -- "$(dirname -- "$0")" && pwd -P)" || exit 1

cargo run --quiet -- update-source
cargo run --quiet -- write-mappings
