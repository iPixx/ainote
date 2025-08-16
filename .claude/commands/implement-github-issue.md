# Enhanced GitHub Issue Implementation Command for aiNote Project

Please analyze and implement the solution for GitHub issue: $ARGUMENTS.

**COMMAND BEHAVIOR:**
- **Complex issues (6+ complexity points):** Creates sub-issues ONLY, no implementation
- **Simple issues or sub-issues:** Proceeds with full implementation
- **Sub-issues:** Cannot create additional sub-issues (prevents recursion)

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

**If HIGH COMPLEXITY (6+ points), proceed to Sub-Issue Creation ONLY**

## 1.1. Sub-Issue Creation Process (For High Complexity Issues)

**IMPORTANT:** When sub-issues are required, this command will ONLY create sub-issues and NOT implement the original issue. Implementation happens separately for each sub-issue.

When an issue scores 6+ complexity points, create manageable sub-issues:

### Sub-Issue Planning Strategy

1. **Break down by logical components:**
   - Backend (Rust/Tauri commands)
   - Frontend (JavaScript/HTML/CSS)
   - Configuration and setup
   - Testing and validation
   - Documentation updates

2. **Create focused sub-issues using `gh issue create`:**

   ```bash
   # Create each sub-issue with proper linking
   gh issue create \
     --title "[Sub-issue 1/N] {Component}: {Specific task}" \
     --body "Part of #{original-issue-number}

   ## Scope
   {Specific deliverable for this sub-issue}

   ## Acceptance Criteria
   - [ ] {Specific testable criteria}
   - [ ] {Integration with other components}

   ## Dependencies
   - Blocks: #{original-issue-number}
   - Related: #{other-sub-issue-numbers}

   ## Implementation Notes
   {Technical details specific to this component}"
   ```

3. **Update original issue with sub-issue links:**
   
   Use `gh issue edit $ARGUMENTS` to add sub-issue tracking:
   
   ```markdown
   ## Implementation Breakdown
   
   This issue has been split into manageable sub-issues:
   
   - [ ] #{sub-issue-1} - Backend implementation
   - [ ] #{sub-issue-2} - Frontend components  
   - [ ] #{sub-issue-3} - Integration testing
   - [ ] #{sub-issue-4} - Documentation
   
   **Progress:** 0/4 sub-issues completed
   ```

4. **Set proper issue relationships:**
   - Add "epic" label to original issue
   - Add "sub-task" label to sub-issues
   - Use GitHub's task lists for tracking
   - Link sub-issues in original issue description

### Sub-Issue Creation Guidelines

- **Each sub-issue should:**
  - Be completable in 4-8 hours
  - Have clear, testable acceptance criteria
  - Reference the parent issue
  - Include specific technical implementation notes
  - Have a single, focused responsibility

- **Naming convention:**
  ```
  [Sub-issue X/N] {Component}: {Action} - {Brief description}
  ```
  
  Examples:
  - `[Sub-issue 1/4] Backend: Implement file tree scanning commands`
  - `[Sub-issue 2/4] Frontend: Create interactive file tree component`

### After Sub-Issue Creation

- **STOP HERE - Do not implement the original issue**
- **Use this command again with each sub-issue number to implement individually**
- **Each sub-issue implementation is a separate command execution**
- **Example workflow:**
  ```bash
  # Step 1: Create sub-issues for complex issue
  ./implement-github-issue 123  # Creates sub-issues 124, 125, 126, 127

  # Step 2: Implement each sub-issue separately (with full context)
  ./implement-github-issue 124  # Gathers context from #123 + all sub-issues, then implements
  ./implement-github-issue 125  # Gathers context from #123 + all sub-issues, then implements
  ./implement-github-issue 126  # Gathers context from #123 + all sub-issues, then implements
  ./implement-github-issue 127  # Gathers context from #123 + all sub-issues, then implements
  ```

---

## 1.2. Sub-Issue Detection (Prevent Recursive Sub-Issue Creation)

**Before any complexity assessment, check if this is already a sub-issue:**

1. **Check issue title for sub-issue pattern:**
   ```bash
   # Look for "[Sub-issue X/N]" pattern in title
   gh issue view $ARGUMENTS --json title
   ```

2. **Check issue body for parent issue reference:**
   ```bash
   # Look for "Part of #" pattern in body
   gh issue view $ARGUMENTS --json body
   ```

3. **If this IS a sub-issue:**
   - **SKIP complexity assessment**
   - **SKIP sub-issue creation**
   - **Proceed to comprehensive context gathering (Step 1.3)**
   - **No further sub-issues can be created**

---

## 1.3. Comprehensive Sub-Issue Context Gathering

**CRITICAL:** When implementing a sub-issue, you MUST gather complete context from the main issue and ALL related sub-issues before proceeding with implementation.

### Context Gathering Process

1. **Extract Parent Issue Information:**
   ```bash
   # Parse the parent issue number from sub-issue body
   PARENT_ISSUE=$(gh issue view $ARGUMENTS --json body | jq -r '.body' | grep -o "Part of #[0-9]*" | grep -o "[0-9]*")
   echo "Parent issue: #$PARENT_ISSUE"
   ```

2. **Fetch Main Issue Context:**
   ```bash
   # Get complete parent issue details
   gh issue view $PARENT_ISSUE --json title,body,labels,assignees,milestone
   ```

3. **Identify ALL Related Sub-Issues:**
   ```bash
   # Search for all sub-issues referencing the parent issue
   gh issue list --search "Part of #$PARENT_ISSUE" --json number,title,state,body
   ```

4. **Analyze Sub-Issue Dependencies:**
   
   For each related sub-issue, check:
   - Current implementation status (open/closed)
   - Dependencies mentioned in acceptance criteria
   - Blocking relationships ("Blocks:", "Depends on:")
   - Integration points with current sub-issue
   - Shared components or files

5. **Create Implementation Context Map:**

   Document the following before proceeding:
   
   ```markdown
   ## Sub-Issue Implementation Context
   
   **Current Sub-Issue:** #{current-issue} - {title}
   **Parent Issue:** #{parent-issue} - {title}
   
   ### Related Sub-Issues Status:
   - [ ] #{sub-issue-1} - {status} - {brief-description}
   - [ ] #{sub-issue-2} - {status} - {brief-description}
   - [ ] #{sub-issue-3} - {status} - {brief-description}
   
   ### Dependencies for Current Implementation:
   - **Blocks:** {what this sub-issue blocks}
   - **Depends on:** {what must be completed first}
   - **Integrates with:** {which sub-issues share components}
   
   ### Shared Components/Files:
   - {file/component} - also modified by #{other-sub-issue}
   - {file/component} - depends on #{prerequisite-sub-issue}
   
   ### Implementation Constraints:
   - {constraint from parent issue}
   - {constraint from related sub-issues}
   - {integration requirements}
   ```

### Context Validation Checklist

Before proceeding to implementation, verify:

- [ ] **Parent issue fully understood** - requirements, acceptance criteria, constraints
- [ ] **All related sub-issues identified** - complete list of sibling sub-issues
- [ ] **Dependencies mapped** - what must be done first, what this blocks
- [ ] **Integration points clear** - how this sub-issue connects with others
- [ ] **Shared resources identified** - files/components touched by multiple sub-issues
- [ ] **Implementation order validated** - this sub-issue can be safely implemented now
- [ ] **Conflict prevention** - no overlap with other sub-issues
- [ ] **Testing strategy** - how to test without breaking other sub-issues

### Critical Implementation Rules for Sub-Issues

1. **Dependency Order:** Never implement a sub-issue that depends on unopened/incomplete sub-issues
2. **Shared File Safety:** If multiple sub-issues modify the same file, coordinate carefully
3. **Integration Points:** Ensure changes don't break interfaces expected by other sub-issues
4. **Testing Isolation:** Test changes in isolation and in context of parent issue
5. **Progress Updates:** Update parent issue progress tracking after completion

### Error Conditions - STOP Implementation If:

- **Parent issue not found or inaccessible**
- **Critical dependency sub-issues not yet implemented**
- **Shared file conflicts detected with other open sub-issues**
- **Implementation would break interfaces needed by other sub-issues**
- **Insufficient context to understand full integration requirements**

### Example Context Gathering Output:

```bash
# Example for sub-issue implementing frontend file tree
Parent Issue: #123 - Implement complete file tree navigation system
Related Sub-Issues:
- #124 [Sub-issue 1/4] Backend: File tree scanning commands (COMPLETED)
- #125 [Sub-issue 2/4] Frontend: File tree component (CURRENT - IN PROGRESS)  
- #126 [Sub-issue 3/4] Integration: Editor-tree communication (PENDING)
- #127 [Sub-issue 4/4] Testing: E2E file navigation tests (PENDING)

Dependencies:
- Depends on: #124 (backend commands) - ✅ COMPLETED
- Blocks: #126 (needs tree component interface)
- Integrates with: #127 (testing will validate component)

Shared Files:
- src/main.js - backend integration points
- src/styles.css - component styling
- No conflicts detected with other open sub-issues
```

---

## 2. Branch Management & Setup (CRITICAL FIRST STEP)

**⚠️ MANDATORY: Create a new branch BEFORE any implementation work begins**

### Branch Creation Process

1. **Check current git status:**
   ```bash
   git status
   git branch -v
   ```

2. **Ensure clean working directory:**
   ```bash
   # Stash any uncommitted changes
   git stash push -m "Pre-issue-$ARGUMENTS work in progress"
   ```

3. **Switch to main branch and pull latest:**
   ```bash
   git checkout main
   git pull origin main
   ```

4. **Create and checkout new branch:**
   ```bash
   # Branch naming convention: issue-{number}-{short-kebab-case-description}
   git checkout -b issue-$ARGUMENTS-$(gh issue view $ARGUMENTS --json title | jq -r '.title' | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9]/-/g' | sed 's/--*/-/g' | sed 's/^-\|-$//g' | cut -c1-50)
   ```

5. **Verify branch creation:**
   ```bash
   git branch -v
   echo "✅ New branch created: $(git branch --show-current)"
   ```

### Branch Naming Examples
- Issue #123 "Implement file tree component" → `issue-123-implement-file-tree-component`
- Issue #45 "Fix editor preview toggle bug" → `issue-45-fix-editor-preview-toggle-bug`
- Sub-issue #67 "[Sub-issue 2/4] Frontend: Add syntax highlighting" → `issue-67-frontend-add-syntax-highlighting`

### Error Handling for Branch Creation
- **If branch already exists:** Use `issue-$ARGUMENTS-v2`, `issue-$ARGUMENTS-v3`, etc.
- **If uncommitted changes:** Force stash and proceed, notify about stashed changes
- **If main branch outdated:** Pull latest changes before creating branch
- **If network issues:** Continue with local branch, warn about potential conflicts

### Critical Rules
- **NEVER implement on main branch**
- **NEVER implement without creating a branch first**
- **ALWAYS base new branch on latest main**
- **ALWAYS verify clean working state before branching**

## 3. Project Context Validation

- Read README.md, CONTRIBUTING.md, and CLAUDE.md for project constraints
- Validate implementation approach against local-first principles
- Ensure solution aligns with lightweight design requirements (100MB memory target)
- Check compatibility with vanilla JS/Rust-only technology stack
- Consider impact on future AI integration (Ollama)

## 4. Comprehensive Codebase Analysis

- Search for existing implementations related to the issue
- Identify affected files, functions, and components
- Map dependencies and integration points
- Check for existing tests that might be affected
- Review similar patterns in the codebase for consistency
- Document current state vs. required changes

## 5. Implementation Planning

- Create detailed implementation strategy
- Identify required changes in both Rust backend and JavaScript frontend
- Plan Tauri command modifications if needed
- Consider edge cases and error handling scenarios
- Estimate complexity and potential risks
- Plan for backward compatibility

## 6. Implementation & Development

- Implement changes following project coding standards
- Maintain vanilla JavaScript (no frameworks) for frontend
- Use minimal Rust dependencies for backend
- Ensure proper error handling with `Result<T>` types
- Add comprehensive JSDoc comments for JavaScript
- Follow existing code patterns and conventions
- Implement incrementally with frequent testing

## 7. Testing & Validation

- Write unit tests for new functionality (Rust: `cargo test`)
- Create integration tests for Tauri commands
- Test frontend-backend communication
- Perform manual testing with realistic data
- Test cross-platform compatibility if applicable
- Verify performance targets (file operations <50ms, memory <100MB)
- Test with large vaults (100+ files) if relevant

## 8. Quality Assurance

- Run `cargo check` for Rust compilation errors
- Run `cargo clippy` for Rust linting and suggestions
- Run `cargo test` to ensure all tests pass
- Test in development mode: `pnpm tauri dev`
- Verify no regressions in existing functionality
- Check for memory leaks or performance degradation
- Validate against issue acceptance criteria

## 9. Documentation & Code Review

- Update relevant documentation if needed
- Add inline code comments for complex logic
- Ensure code is self-documenting and maintainable
- Review changes against project principles
- Verify no external dependencies were added without approval

## 10. Commit & PR Creation

**⚠️ VERIFY: Ensure you're on the correct feature branch before committing**

```bash
# Verify current branch
echo "Current branch: $(git branch --show-current)"
# Should show: issue-{number}-{description}, NOT main
```

- Stage changes: `git add .`
- Create descriptive commit message following format:

  ```
  {type}(scope): {description}

  {detailed explanation if needed}

  Fixes #{issue-number}
  ```

- Push branch: `git push -u origin $(git branch --show-current)`
- Create PR using `gh pr create` with comprehensive description:
  - Link to original issue
  - Summary of changes made
  - Testing performed
  - Any breaking changes or migration notes

## 11. Post-Implementation Validation

- Verify PR creation and proper issue linking
- Check that all CI/CD checks pass (if configured)
- Ensure PR description includes testing checklist
- Confirm issue will be automatically closed on merge
- **Verify PR is NOT targeting main with uncommitted changes from main branch**

### Additional Steps for Sub-Issue Completion

When completing a sub-issue, also perform:

1. **Update Parent Issue Progress:**
   ```bash
   # Update the parent issue's sub-issue checklist
   gh issue edit $PARENT_ISSUE --body "$(gh issue view $PARENT_ISSUE --json body | jq -r '.body' | sed "s/- \[ \] #$ARGUMENTS/- [x] #$ARGUMENTS/")"
   ```

2. **Comment on Parent Issue:**
   ```bash
   gh issue comment $PARENT_ISSUE --body "✅ Sub-issue #$ARGUMENTS completed and merged. 

   **Implementation Summary:**
   - {brief summary of what was implemented}
   - {key files modified}
   - {integration points established}

   **Next Steps:**
   - {any preparation for dependent sub-issues}
   - {notes for remaining sub-issues}"
   ```

3. **Validate Integration:**
   - Test that changes integrate properly with completed sub-issues
   - Verify no breaking changes for pending sub-issues
   - Update any shared interfaces or contracts

## Error Handling & Edge Cases

**Ask clarifying questions if:**

- Issue number doesn't exist or is inaccessible
- Issue description lacks sufficient detail for implementation
- Required changes conflict with project constraints (external dependencies, frameworks)
- Implementation would violate performance or memory targets
- Changes require breaking existing API or file format compatibility
- Issue has unresolved dependencies or blockers
- **Complex issue lacks sufficient breakdown guidance for sub-issue creation**

**Abort implementation if:**

- **Branch creation fails or user is working on main branch**
- Issue requires external dependencies not approved in CONTRIBUTING.md
- Changes would exceed memory constraints (100MB target)
- Implementation conflicts with local-first principles
- Required changes would break existing functionality without migration path
- **Cannot switch to a clean branch due to unresolvable conflicts**

**For Sub-Issue Management:**

- **If sub-issue creation fails:** Continue with monolithic implementation but add detailed task breakdown in comments
- **If original issue becomes stale:** Use `gh issue comment` to provide progress updates
- **If sub-issues conflict:** Resolve dependencies first, then implement in correct order
- **If sub-issue scope creep:** Create additional sub-issues rather than expanding existing ones

## Project-Specific Considerations

- **Performance:** Monitor memory usage during implementation
- **Architecture:** Maintain three-column layout structure
- **Future-proofing:** Consider Ollama integration readiness
- **File Operations:** Ensure efficient file handling for large vaults
- **Cross-platform:** Test on multiple operating systems if relevant
- **Security:** Never expose or log sensitive user data
- **Privacy:** Maintain complete local-first operation
