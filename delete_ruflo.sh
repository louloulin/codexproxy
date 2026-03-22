#!/bin/bash
# Delete all ruflo-related files and directories

echo "Deleting ruflo-related files..."

# Delete .claude-flow directory
if [ -d ".claude-flow" ]; then
    rm -rf .claude-flow
    echo "✓ Deleted .claude-flow directory"
else
    echo "- .claude-flow directory not found"
fi

# Delete ruflo-related helper files
files_to_delete=(
    ".claude/helpers/auto-memory-hook.mjs"
    ".claude/helpers/statusline.cjs"
    ".claude/helpers/statusline.js"
)

for file in "${files_to_delete[@]}"; do
    if [ -f "$file" ]; then
        rm -f "$file"
        echo "✓ Deleted $file"
    else
        echo "- $file not found"
    fi
done

echo ""
echo "Cleanup complete!"
