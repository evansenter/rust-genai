#!/bin/bash
# Stop hook: Provides context for Claude to decide whether to auto-continue
#
# Outputs JSON with current git/PR state. Claude uses this context to determine
# if there's obvious follow-up work (e.g., CI running, pending feedback).

set -e

# Check for recent git/gh activity to determine context
RECENT_PUSH=$(git log --oneline -1 --since="5 minutes ago" 2>/dev/null | head -1)
CURRENT_BRANCH=$(git branch --show-current 2>/dev/null)
PR_NUMBER=$(gh pr view --json number -q .number 2>/dev/null || echo "")

# Function to check if CI is running or just completed
check_ci_status() {
    if [ -n "$PR_NUMBER" ]; then
        # Get the most recent check run status
        STATUS=$(gh pr checks "$PR_NUMBER" --json state -q '.[0].state' 2>/dev/null || echo "")
        echo "$STATUS"
    fi
}

# Function to check for unread PR feedback
check_pr_feedback() {
    if [ -n "$PR_NUMBER" ]; then
        # Count review comments
        # Note: {owner}/{repo} placeholders are auto-resolved by gh CLI from git remote
        COMMENT_COUNT=$(gh api "repos/{owner}/{repo}/pulls/$PR_NUMBER/comments" --jq 'length' 2>/dev/null || echo "0")
        echo "$COMMENT_COUNT"
    else
        echo "0"
    fi
}

# Determine if we should continue
CI_STATUS=$(check_ci_status)
FEEDBACK_COUNT=$(check_pr_feedback)

# Decision logic
# Note: This script provides context; the actual decision uses a prompt hook
# that can reason about whether to continue

# Output context for the prompt hook to use
cat << EOF
{
  "context": {
    "recent_push": "$RECENT_PUSH",
    "current_branch": "$CURRENT_BRANCH",
    "pr_number": "$PR_NUMBER",
    "ci_status": "$CI_STATUS",
    "feedback_count": "$FEEDBACK_COUNT"
  }
}
EOF
