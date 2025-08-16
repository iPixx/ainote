# Enhanced GitHub Issues Creation Command for aiNote Project Phases

Please analyze the project documentation and create comprehensive GitHub issues for phase $ARGUMENTS.

Follow these steps:

## 1. Project Analysis & Validation
- Read and analyze README.md, CONTRIBUTING.md, and CLAUDE.md to understand the project architecture and phase requirements
- Validate that the requested phase exists and understand its scope and dependencies
- Identify completion criteria and success metrics for the phase

## 2. GitHub Repository Setup
- Create milestone in GitHub for the specified phase with appropriate description and due date
- Set milestone description with phase objectives and key deliverables

## 3. Codebase Assessment
- Search the entire codebase to understand current implementation status
- Identify existing components, functions, and infrastructure that relate to the phase
- Document what's already implemented vs. what needs to be built
- Check for any blockers or dependencies from previous phases

## 4. Implementation Planning
- Create a detailed implementation plan markdown file (e.g., `phase-{N}-implementation-plan.md`)
- Include technical specifications, architecture decisions, and integration points
- Define specific deliverables, acceptance criteria, and testing requirements
- Consider performance constraints (100MB memory target, AI resource allocation)
- Plan for local-first and lightweight implementation principles

## 5. Issue Creation & Organization
- Break down the implementation plan into specific, actionable GitHub issues
- Ensure each issue follows the format:
  - Clear title and description
  - Detailed acceptance criteria
  - Required unit tests and integration tests
  - Performance benchmarks where applicable
  - Dependencies on other issues
  - Estimated effort/complexity
- Attach all issues to the created milestone
- Add appropriate labels (enhancement, testing, documentation, etc.)
- Set up issue dependencies and ordering

## 6. Quality Assurance
- Ensure all issues include specific unit test requirements
- Verify test coverage expectations are clearly defined
- Include performance testing requirements where relevant
- Add integration testing scenarios for Tauri frontend-backend communication

## 7. Documentation & Communication
- Create or update project documentation to reflect the planned work
- Ensure the implementation plan aligns with project principles (vanilla JS, Rust minimal deps, local-first)
- Validate against memory and performance constraints

## Additional Considerations:
- Use GitHub CLI (`gh`) for all GitHub operations
- Consider cross-platform compatibility (Windows, macOS, Linux)
- Plan for future AI integration (Ollama) even in early phases
- Ensure backward compatibility with existing vault files
- Include accessibility and user experience considerations

## Ask clarifying questions if:
- The requested phase number is unclear or doesn't exist
- There are ambiguities in the phase requirements
- Dependencies between phases need clarification
- Specific technical constraints need to be addressed
