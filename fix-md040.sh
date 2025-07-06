#!/bin/bash

# Fix MD040 errors - missing code fence languages

echo "=== Fixing MD040 errors ==="

# Replace pseudocode with rust (since this is a Rust project)
find ms-framework-docs -name "*.md" -type f -exec sed -i '' 's/```pseudocode/```rust/g' {} \;

# For empty code fences, we need to analyze the content
# Let's first get all files with empty code fences
echo "Finding files with empty code fences..."

# Create a list of files with empty code fences
markdownlint-cli2 ms-framework-docs/**/*.md --config .markdownlint.json 2>&1 | \
  grep 'MD040.*Context: "```"' | \
  cut -d':' -f1 | \
  sort -u > files_with_empty_fences.txt

echo "Files with empty code fences:"
cat files_with_empty_fences.txt

# Count remaining MD040 errors
echo -e "\n=== Checking remaining MD040 errors ==="
REMAINING=$(markdownlint-cli2 ms-framework-docs/**/*.md --config .markdownlint.json 2>&1 | grep -c "MD040" || echo "0")
echo "Remaining MD040 errors: $REMAINING"