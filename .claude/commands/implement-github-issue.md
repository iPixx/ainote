# GitHub Issue Implementation Command for aiNote Project

Analyze and implement solution for GitHub issue: $ARGUMENTS.

**BEHAVIOR:**
- **High complexity (6+ points):** Create minimal necessary sub-issues, no implementation
- **Low complexity or sub-issues:** Full implementation
- **Sub-issues:** Cannot create additional sub-issues

Follow these steps:

## 1. Issue Analysis & Complexity Assessment

- Use `gh issue view $ARGUMENTS` to get detailed issue information
- Parse issue description, acceptance criteria, and labels
- Identify issue type (bug fix, feature, enhancement, documentation)
- Check for linked dependencies, blockers, or related issues
- Understand the expected deliverables and success metrics

### Complexity Assessment Criteria

Evaluate if the issue is **HIGH COMPLEXITY** based on these factors:

**High Complexity Indicators (3+ points = HIGH):**
- Multiple components affected (frontend + backend + config) - 2 points
- New feature requiring significant architecture changes - 3 points
- Breaking changes or migration requirements - 3 points
- Cross-platform compatibility concerns - 2 points
- Performance optimization requirements - 2 points
- Integration with external systems (Ollama, file system) - 2 points
- More than 5 files to be modified - 2 points
- Estimated implementation time > 1 day - 2 points
- Complex business logic or algorithms - 2 points
- Security or privacy implications - 2 points

**If HIGH COMPLEXITY (6+ points), create sub-issues ONLY**

## 1.1. Sub-Issue Creation (High Complexity Only)

When complexity ≥6 points, create minimal necessary sub-issues based on logical separation:

**Planning:** Identify distinct, independent components that can be implemented separately
**Creation:** Use `gh issue create` for each sub-issue:
```bash
gh issue create \
  --title "[Sub-issue X/N] {Component}: {Task}" \
  --body "Part of #$ARGUMENTS

## Scope
{Specific deliverable}

## Acceptance Criteria  
- [ ] {Testable criteria}

## Dependencies
- Blocks: #$ARGUMENTS"
```

**Update original:** Add sub-issue tracking to original issue via `gh issue edit`

**Guidelines:**
- Each sub-issue: 4-8 hours, single responsibility, clear acceptance criteria
- Naming: `[Sub-issue X/N] {Component}: {Action}`
- **STOP after creation** - implement each sub-issue separately

---

## 1.2. Sub-Issue Detection

Before complexity assessment, check if issue is already a sub-issue:
- Title contains `[Sub-issue X/N]` pattern
- Body contains `Part of #` reference

**If sub-issue:** Skip complexity assessment, proceed to context gathering

---

## 1.3. Sub-Issue Context Gathering

For sub-issues, gather context from parent issue and related sub-issues:
1. Extract parent issue number from body
2. Fetch parent issue details
3. List all related sub-issues 
4. Check dependencies and shared components
5. Verify implementation order and integration points

**Validation:** Verify parent context, dependencies, integration points, and no conflicts

**Critical Rules:**
- Check dependency order before implementing
- Coordinate shared file modifications 
- Test in isolation and with parent context
- Update parent issue progress after completion

**Stop if:** Missing parent context, unmet dependencies, or file conflicts detected

---

## 2. Branch Management (MANDATORY)

**⚠️ Create branch BEFORE implementation:**

```bash
# Clean and update
git stash push -m "Pre-issue-$ARGUMENTS"
git checkout main && git pull origin main

# Create branch: issue-{number}-{description}
git checkout -b issue-$ARGUMENTS-{generated-name}
```

**Rules:** Never work on main, always branch from latest main, handle conflicts appropriately

## 3. Context & Analysis
- Review README.md, CONTRIBUTING.md, CLAUDE.md for constraints
- Search existing implementations and patterns
- Map affected files, dependencies, integration points
- Plan implementation strategy for frontend/backend changes

## 4. Implementation
- Follow vanilla JS/Rust standards, no external dependencies
- Use `Result<T>` error handling, JSDoc comments
- Implement incrementally with frequent testing
- Maintain performance targets (<50ms operations, <100MB memory)

## 5. Testing & Quality
```bash
cargo check && cargo clippy && cargo test
pnpm tauri dev  # Manual testing
```
- Unit tests, integration tests, cross-platform validation
- Verify no regressions, check against acceptance criteria

## 6. Commit & PR
**⚠️ Verify on feature branch (not main):**
```bash
git add .
git commit -m "{type}(scope): {description}

Fixes #$ARGUMENTS"
git push -u origin $(git branch --show-current)
gh pr create  # Link issue, summarize changes, testing notes
```

**Sub-issue completion:** Update parent issue progress and add completion comment

## 7. Error Handling

**Ask questions if:** Issue unclear, conflicts with constraints, missing dependencies
**Abort if:** Can't create branch, violates memory/performance targets, breaks local-first principles  
**Sub-issue issues:** Create additional sub-issues for scope creep, resolve conflicts by dependency order

## Project Constraints
- Performance: <100MB memory, <50ms operations
- Architecture: Three-column layout, local-first
- Security: No data exposure/logging, complete privacy
