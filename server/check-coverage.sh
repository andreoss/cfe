#!/bin/sh
set -e
cd "$(dirname "$0")"
cargo llvm-cov -p domain -p app --fail-under-lines 85
