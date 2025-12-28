#!/bin/sh
set -e
cd "$(dirname "$0")"
cargo llvm-cov -p domain -p app -p front --fail-under-lines 85
