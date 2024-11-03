#!/bin/bash
file="$1"
echo "$file"
file="$file" script -fqc 'cargo run -q < "$file"' /dev/null | tr -d \\r > fixtures/output/"$(basename "$file")"
