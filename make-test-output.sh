#!/bin/bash
file="$1"
echo "$file"
cargo build
file="$file" script -fqc 'target/debug/dyff < "$file"' /dev/null | tr -d \\r > fixtures/output/"$(basename "$file")"
