#!/bin/sh
set -e
cd "$(dirname "$0")"
cargo llvm-cov --fail-under-lines 85
