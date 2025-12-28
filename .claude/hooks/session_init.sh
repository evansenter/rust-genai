#!/bin/bash
# Session initialization hook for rust-genai project
#
# Provides context about recent project activity at session start

set -e

echo "=== rust-genai Session Init ==="
echo ""

# Check environment
if [ -z "$GEMINI_API_KEY" ]; then
    echo "WARNING: GEMINI_API_KEY not set - integration tests will fail"
    echo ""
fi

# Recent PRs (last 5)
echo "### Recent Pull Requests"
echo ""
PR_LIST=$(gh pr list --limit 5 --json number,title,state,updatedAt,author --template '{{range .}}#{{.number}} [{{.state}}] {{.title}} ({{.author.login}}, {{timeago .updatedAt}})
{{end}}' 2>/dev/null || echo "Unable to fetch PRs")

if [ -n "$PR_LIST" ] && [ "$PR_LIST" != "Unable to fetch PRs" ]; then
    echo "$PR_LIST"
else
    echo "No recent PRs or unable to fetch"
fi
echo ""

# Open Issues (last 5)
echo "### Open Issues"
echo ""
ISSUE_LIST=$(gh issue list --limit 5 --json number,title,labels,updatedAt --template '{{range .}}#{{.number}} {{.title}}{{if .labels}} [{{range $i, $l := .labels}}{{if $i}}, {{end}}{{$l.name}}{{end}}]{{end}} ({{timeago .updatedAt}})
{{end}}' 2>/dev/null || echo "Unable to fetch issues")

if [ -n "$ISSUE_LIST" ] && [ "$ISSUE_LIST" != "Unable to fetch issues" ]; then
    echo "$ISSUE_LIST"
else
    echo "No open issues or unable to fetch"
fi
echo ""

# Recent commits (last 3)
echo "### Recent Commits"
echo ""
git log --oneline -3 2>/dev/null || echo "Unable to fetch commits"
echo ""

# Current branch status
BRANCH=$(git branch --show-current 2>/dev/null || echo "unknown")
echo "### Current State"
echo "Branch: $BRANCH"

# Check if there's an open PR for current branch
if [ "$BRANCH" != "main" ] && [ "$BRANCH" != "unknown" ]; then
    PR_FOR_BRANCH=$(gh pr view --json number,state -q '"PR #\(.number) [\(.state)]"' 2>/dev/null || echo "No PR")
    echo "PR: $PR_FOR_BRANCH"
fi

# Quick build check
echo ""
echo "### Build Status"
cargo check --quiet 2>/dev/null && echo "Build: OK" || echo "Build: FAILING"

echo ""
echo "=== Ready ==="
