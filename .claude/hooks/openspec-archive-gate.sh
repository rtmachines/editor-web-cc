#!/usr/bin/env bash
# PreToolUse gate for the Bash and PowerShell tools.
# The gate runs "cargo test --workspace" before an "openspec archive" command.
# Exit 0 lets the tool call run. Exit 2 blocks it, and Claude reads stderr.
# Source of the rule: CLAUDE.md, section Workflow, and .claude/rules/openspec.md.
set -u

input="$(cat)"

# Fast path: most tool calls do not mention the archive command.
case "$input" in
  *openspec*archive*) ;;
  *) exit 0 ;;
esac

# Read the command text from the hook payload.
# When jq is missing or cannot parse the payload, match the raw payload, so the gate fails closed.
command_text=""
if command -v jq >/dev/null 2>&1; then
  command_text="$(printf '%s' "$input" | jq -r '.tool_input.command // empty' 2>/dev/null)" || command_text=""
fi
if [ -z "$command_text" ]; then
  command_text="$input"
fi

# Run the gate only when the command calls "openspec archive".
if ! printf '%s' "$command_text" | grep -Eq '(^|[^[:alnum:]_-])openspec[[:space:]]+archive([[:space:]]|$)'; then
  exit 0
fi

project_dir="${CLAUDE_PROJECT_DIR:-$(pwd)}"
if ! cd "$project_dir"; then
  echo "openspec archive is blocked: the gate cannot open the project directory $project_dir." >&2
  exit 2
fi

test_log="$(cargo test --workspace 2>&1)"
test_status=$?

if [ "$test_status" -ne 0 ]; then
  {
    echo "openspec archive is blocked: cargo test --workspace failed with exit code $test_status."
    echo "Make the tests pass, then run the archive again."
    echo "Last 40 lines of the test output:"
    printf '%s\n' "$test_log" | tail -n 40
  } >&2
  exit 2
fi

exit 0
