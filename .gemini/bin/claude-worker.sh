#!/usr/bin/env bash
# Usage: claude-worker.sh <agent: reviewer|explorer|builder> <task_prompt> [optional_model] [optional_effort]

AGENT="$1"
TASK="$2"
MODEL_ARGS=()
[ -n "$3" ] && MODEL_ARGS+=(--model "$3")
[ -n "$4" ] && MODEL_ARGS+=(--effort "$4")

claude -p "$TASK" \
  --agent "$AGENT" \
  --no-session-persistence \
  "${MODEL_ARGS[@]}"
