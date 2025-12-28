# PR Feedback Review

Process and respond to PR review feedback with critical thinking.

## Usage

```
/pr-feedback [PR_NUMBER]
```

If PR_NUMBER is omitted, uses the current branch's PR.

## Instructions

When this skill is invoked:

### 1. Fetch Review Comments

```bash
# Get PR number if not provided
PR_NUM="${1:-$(gh pr view --json number -q .number 2>/dev/null)}"

# Fetch all review comments
gh api repos/{owner}/{repo}/pulls/$PR_NUM/comments
gh api repos/{owner}/{repo}/pulls/$PR_NUM/reviews
gh api repos/{owner}/{repo}/issues/$PR_NUM/comments
```

### 2. Categorize Feedback

Classify each piece of feedback:

- **Critical**: Security issues, bugs, broken functionality, missing error handling
- **Important**: Test coverage gaps, API design issues, documentation gaps, code quality
- **Suggestions**: Style preferences, minor refactors, nice-to-haves, optimizations

### 3. Form Opinions

For each item, assess:
- Does this feedback understand the context and purpose of the change?
- Is this a genuine improvement or unnecessary complexity?
- Does implementing this align with project conventions (check CLAUDE.md)?
- Is the effort proportional to the benefit?

### 4. Present Opinion Table

Output a table with your honest assessment:

```markdown
## PR Feedback Review

| # | Severity | Feedback | File:Line | Opinion | Action |
|---|----------|----------|-----------|---------|--------|
| 1 | Critical | [summary] | path:123 | Agree - [reason] | Implement |
| 2 | Important | [summary] | path:456 | Disagree - [reason] | **Discuss** |
| 3 | Suggestion | [summary] | path:789 | Agree but trivial | Skip |

### Items Requiring Discussion

[For any Critical/Important items marked "Discuss", explain your reasoning in detail]

### Implementation Plan

[List items you will implement immediately]
```

### 5. Implementation Rules

**Implement immediately** (no discussion needed):
- Critical items you agree with
- Important items you agree with
- Suggestions you agree with AND are trivial (<5 lines)

**Stop and discuss** (wait for user input):
- Critical items you disagree with or are uncertain about
- Important items you disagree with or are uncertain about
- Any feedback that seems to misunderstand the purpose of the change

**Skip** (note in summary but don't implement):
- Suggestions you disagree with
- Out-of-scope feedback (create GitHub issue instead)
- Feedback already addressed

### 6. After Discussion

Once the user provides input on disputed items:
- Implement items where you reached agreement
- Skip items the user agrees to skip
- Create issues for items deferred to future work

### 7. Push and Re-check

After implementing feedback:
1. Run quality gates: `cargo fmt && cargo clippy && cargo test`
2. Commit with message referencing the feedback addressed
3. Push changes
4. Re-run `/pr-feedback` to verify no new comments

## Key Principle

You have context on the work's purpose that automated reviewers lack. If feedback seems to miss the point, add unnecessary complexity, or conflict with project conventions, flag it for discussion rather than blindly implementing. Honest disagreement is more valuable than compliance.
