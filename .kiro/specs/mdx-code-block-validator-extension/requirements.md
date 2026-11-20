# Requirements Document: MDX Code Block Validator Extension

## Introduction

The documentation site contains code examples in multiple languages embedded within MDX files as code fences. Currently, only TypeScript and JavaScript code blocks are validated for syntax errors during the build process. Shell commands and JSON payloads are equally prone to typos and formatting errors that readers will copy-paste directly from the documentation. This feature extends the existing code block validator to validate JSON and shell script code fences, providing the same error reporting (file path and line number) for all supported block types. The solution integrates seamlessly with the existing validation infrastructure to catch errors early and prevent documentation readers from copying broken examples.

## Glossary

- **CodeBlockValidator**: The build-time tool that validates code blocks in MDX files for syntax errors
- **Code_Fence**: A fenced code block in MDX delimited by triple backticks with a language identifier (e.g., ` ```json ... ``` `)
- **Language_Identifier**: The tag after the opening triple backticks that specifies the programming language (e.g., `json`, `sh`, `bash`, `shell`, `ts`, `js`)
- **JSON_Block**: A code fence with language identifier `json` containing JSON data structures
- **Shell_Block**: A code fence with language identifiers `sh`, `bash`, or `shell` containing shell script commands
- **Valid_JSON**: A JSON string that parses successfully according to the JSON specification (RFC 8259), with no trailing commas or comments
- **Valid_Shell**: A shell script that can be parsed without syntax errors by a shell linter for bash/sh compatibility
- **MDX_File**: A markdown file with embedded JSX located in `docs/app/docs/**/*.mdx`
- **Line_Number**: The line within an MDX file where a code fence begins (1-indexed)
- **Error_Report**: A structured output message containing the source file path, line number, and description of the validation error
- **Build_Pipeline**: The Next.js build process triggered by `npm run build` or CI/CD systems
- **Existing_Infrastructure**: The current TypeScript and JavaScript code block validation system that this feature extends
- **Shell_Linter**: A tool that parses and validates shell script syntax (e.g., ShellCheck)
- **JSON_Parser**: A JSON parser that validates JSON syntax and structure according to RFC 8259

## Requirements

### Requirement 1: Discover JSON Code Blocks

**User Story:** As a documentation contributor, I want JSON code blocks in my MDX files to be discovered and validated automatically, so that broken JSON examples are caught during the build.

#### Acceptance Criteria

1. WHEN the CodeBlockValidator scans an MDX file, THE CodeBlockValidator SHALL identify all code fences with language identifier `json`
2. WHEN a `json` code fence is found, THE CodeBlockValidator SHALL extract the complete content between the opening and closing backticks
3. WHEN a `json` code fence is found, THE CodeBlockValidator SHALL record the line number where the code fence begins in the source MDX file
4. WHEN the CodeBlockValidator encounters multiple `json` code fences in a single file, THE CodeBlockValidator SHALL validate each block independently and report errors for each block separately

### Requirement 2: Discover Shell Code Blocks

**User Story:** As a documentation contributor, I want shell and bash code blocks in my MDX files to be discovered and validated automatically, so that broken shell examples are caught during the build.

#### Acceptance Criteria

1. WHEN the CodeBlockValidator scans an MDX file, THE CodeBlockValidator SHALL identify all code fences with language identifiers `sh`, `bash`, or `shell`
2. WHEN a shell code fence is found (any of the three identifiers), THE CodeBlockValidator SHALL extract the complete content between the opening and closing backticks
3. WHEN a shell code fence is found, THE CodeBlockValidator SHALL record the line number where the code fence begins in the source MDX file
4. WHEN the CodeBlockValidator encounters multiple shell code fences in a single file, THE CodeBlockValidator SHALL validate each block independently and report errors for each block separately

### Requirement 3: Validate JSON Syntax

**User Story:** As a documentation maintainer, I want JSON code blocks to be validated for correct JSON syntax, so that readers don't copy-paste invalid JSON from the documentation.

#### Acceptance Criteria

1. WHEN the CodeBlockValidator processes a `json` code fence, THE CodeBlockValidator SHALL parse the content as JSON
2. WHEN JSON is valid (parses successfully with no syntax errors), THE CodeBlockValidator SHALL mark the block as valid and proceed to the next block
3. IF JSON is invalid (contains syntax errors), THEN THE CodeBlockValidator SHALL record the validation error with a description of the JSON parsing error
4. WHEN JSON contains trailing commas (a common error), THEN THE CodeBlockValidator SHALL report an error indicating trailing commas are not allowed in JSON
5. WHEN JSON contains comments (which are not valid in JSON), THEN THE CodeBlockValidator SHALL report an error indicating JSON does not support comments

### Requirement 4: Validate Shell Syntax

**User Story:** As a documentation maintainer, I want shell code blocks to be validated for correct shell syntax, so that readers don't copy-paste broken shell commands from the documentation.

#### Acceptance Criteria

1. WHEN the CodeBlockValidator processes a shell code fence, THE CodeBlockValidator SHALL lint the content for shell syntax errors
2. WHEN shell syntax is valid (contains no linting errors), THE CodeBlockValidator SHALL mark the block as valid and proceed to the next block
3. IF shell syntax is invalid (contains syntax errors), THEN THE CodeBlockValidator SHALL record the validation error with a description of the shell linting error
4. WHEN shell code contains undefined variable references, THE CodeBlockValidator MAY report warnings (if the linter supports this) or skip this check (configurable)
5. WHEN shell code is intentionally incomplete or includes shell pseudo-code placeholders (e.g., `${API_KEY}`), WHERE shell pseudo-code ignoring is enabled, THE CodeBlockValidator SHALL allow these patterns without reporting errors

### Requirement 5: Integrate with Existing TypeScript/JavaScript Validation

**User Story:** As a developer, I want JSON and shell validation to work alongside the existing TypeScript and JavaScript validation without conflicts, so that all code blocks are validated consistently in a single pass.

#### Acceptance Criteria

1. THE CodeBlockValidator extension SHALL use the same code fence discovery and scanning logic as the existing TypeScript/JavaScript validator
2. WHEN the CodeBlockValidator runs, THE CodeBlockValidator SHALL validate all supported language types (TypeScript, JavaScript, JSON, shell) in a single execution
3. WHEN validation completes, THE CodeBlockValidator SHALL combine all errors from all language types into a single report
4. THE CodeBlockValidator extension SHALL not modify the existing TypeScript and JavaScript validation behavior
5. WHEN new language types are added in the future, THE CodeBlockValidator architecture SHALL support adding validators without refactoring core logic

### Requirement 6: Report Errors with File Path and Line Number

**User Story:** As a developer, I want error reports to show exactly where validation errors occur in my MDX files, so that I can fix them quickly.

#### Acceptance Criteria

1. WHEN a code block fails validation, THE CodeBlockValidator SHALL include the source MDX file path (relative to the docs directory or project root)
2. WHEN a code block fails validation, THE CodeBlockValidator SHALL include the line number where the code fence begins (1-indexed)
3. WHEN a code block fails validation, THE CodeBlockValidator SHALL include the language type of the code block (json, bash, etc.)
4. WHEN a code block fails validation, THE CodeBlockValidator SHALL include a description of the validation error
5. THE CodeBlockValidator error report format SHALL be consistent across all language types (TypeScript, JavaScript, JSON, shell)
6. WHEN multiple errors are found in a single MDX file, THE CodeBlockValidator SHALL display errors in line order (ascending by line number)

### Requirement 7: Fail the Build on Validation Errors

**User Story:** As a team lead, I want the build to fail when code block validation errors are detected, so that broken examples never reach production documentation.

#### Acceptance Criteria

1. WHEN the CodeBlockValidator detects one or more validation errors (in any language type), THEN THE CodeBlockValidator SHALL exit with a non-zero exit code
2. WHEN the CodeBlockValidator detects validation errors, THEN THE Build_Pipeline SHALL be interrupted and the build SHALL NOT complete successfully
3. IF no validation errors are found in any code blocks, THEN THE CodeBlockValidator SHALL exit with exit code 0 and allow the build to proceed
4. WHEN the build fails due to validation errors, THE CodeBlockValidator SHALL display all errors in a clear, readable format before failing

### Requirement 8: Configure Block Type Validation

**User Story:** As a project maintainer, I want to configure which code block types are validated and which are skipped, so that validation can be customized to project needs.

#### Acceptance Criteria

1. WHERE a configuration file specifies enabled validators (e.g., `validators: ["json", "shell"]`), THE CodeBlockValidator SHALL only validate those block types
2. WHERE a configuration specifies disabled validators, THE CodeBlockValidator SHALL skip validation for those block types
3. WHERE no configuration exists, THE CodeBlockValidator SHALL use sensible defaults: validate TypeScript, JavaScript, JSON, and shell blocks
4. WHERE a configuration enables shell validation but specifies `ignoreUndefinedVariables: true`, THE CodeBlockValidator SHALL not report errors for undefined variables in shell blocks
5. WHERE a configuration enables shell validation and specifies `linterRules` or similar, THE CodeBlockValidator SHALL apply those rules during linting

### Requirement 9: Configure Ignored Patterns for Shell Blocks

**User Story:** As a documentation writer, I want to mark certain shell commands as pseudo-code or examples without causing validation failures, so that intentional placeholders don't block the build.

#### Acceptance Criteria

1. WHERE a configuration specifies `shellIgnorePatterns` containing a list of regex patterns, THE CodeBlockValidator SHALL skip validation for shell blocks matching any of those patterns
2. WHERE a shell block begins with a special comment like `# pseudo-code` or `# example only`, WHERE this pattern is in `shellIgnorePatterns`, THE CodeBlockValidator SHALL skip validation for that block
3. WHERE a shell block contains variable references like `${API_KEY}` or `${TOKEN}`, WHERE variable pattern ignoring is enabled, THE CodeBlockValidator SHALL not report errors for those references
4. WHEN a shell block is skipped due to a matching ignore pattern, THE CodeBlockValidator SHALL log this decision (if verbose logging is enabled) but not report an error

### Requirement 10: Support JSON Schema Validation (Optional)

**User Story:** As a documentation maintainer, I want JSON code blocks to be validated against optional JSON schemas, so that example payloads match the actual API or configuration structure.

#### Acceptance Criteria

1. WHERE a configuration file specifies JSON schema validation settings, THE CodeBlockValidator MAY read JSON schema files from the specified paths
2. WHERE a JSON code block is tagged with a schema reference (e.g., ` ```json schema=api-payload ``` `), THE CodeBlockValidator MAY validate the JSON against the specified schema
3. WHERE JSON fails schema validation, THE CodeBlockValidator SHALL report the specific schema validation error
4. WHERE schema validation is enabled but a schema file is missing, THE CodeBlockValidator MAY warn but not fail the build (configurable)
5. WHERE schema validation is not configured, THE CodeBlockValidator SHALL perform only basic JSON syntax validation

### Requirement 11: Provide Detailed Error Messages

**User Story:** As a developer fixing broken code blocks, I want clear, actionable error messages that explain what's wrong, so that I can fix issues quickly.

#### Acceptance Criteria

1. WHEN JSON validation fails, THE CodeBlockValidator error message SHALL indicate the specific JSON parsing error (e.g., "Unexpected token at position X" or "Trailing comma in object")
2. WHEN shell validation fails, THE CodeBlockValidator error message SHALL indicate the specific linting error from the shell linter
3. WHEN a JSON block has trailing commas, THE CodeBlockValidator error message SHALL explicitly mention "trailing commas" and suggest the fix
4. WHEN a shell block has syntax errors, THE CodeBlockValidator error message SHALL include the line number within the code block where the error occurs (if available from the linter)
5. THE CodeBlockValidator error messages SHALL include a reference to the line number in the source MDX file for easy navigation

### Requirement 12: Handle Empty and Whitespace-Only Blocks

**User Story:** As a documentation contributor, I want the validator to handle edge cases like empty code blocks gracefully, so that validation doesn't fail on intentional examples.

#### Acceptance Criteria

1. WHEN a code block contains only whitespace (spaces, tabs, newlines), THE CodeBlockValidator SHALL treat it as invalid JSON or shell syntax and report an error
2. WHEN a code block is completely empty (zero bytes between opening and closing backticks), THE CodeBlockValidator SHALL report an error indicating the block is empty
3. WHERE a configuration specifies `allowEmptyBlocks: true`, THE CodeBlockValidator MAY skip validation for empty blocks without reporting an error

### Requirement 13: Support Caching for Build Performance

**User Story:** As a developer, I want the validator to cache previous results and only re-validate changed files, so that incremental builds are fast during development.

#### Acceptance Criteria

1. WHERE caching is enabled in configuration, THE CodeBlockValidator MAY cache validation results from previous runs
2. WHERE a file has not changed since the last successful validation, THE CodeBlockValidator MAY skip validation and reuse cached results
3. WHERE a file has changed, THE CodeBlockValidator SHALL re-validate the entire file
4. THE CodeBlockValidator cache results SHALL be identical to fresh validation results; cached and fresh results MUST match exactly
5. WHERE caching is not explicitly enabled, THE CodeBlockValidator SHALL perform full validation on every run

### Requirement 14: Support Multiple Shell Dialects (Optional)

**User Story:** As a documentation maintainer with cross-platform documentation, I want the validator to support both bash and POSIX sh syntax, so that shell examples work on multiple platforms.

#### Acceptance Criteria

1. WHERE a shell block is tagged with language identifier `bash`, THE CodeBlockValidator MAY validate it as bash-specific syntax
2. WHERE a shell block is tagged with language identifier `sh` or `shell`, THE CodeBlockValidator MAY validate it as POSIX sh syntax
3. WHERE a configuration specifies a `shellDialect` preference (e.g., `bash` or `posix`), THE CodeBlockValidator SHALL use that dialect for validation
4. WHERE different shell dialects have conflicting syntax requirements, THE CodeBlockValidator error message SHALL indicate which dialect caused the error

### Requirement 15: JSON Pretty Printer Validation

**User Story:** As a documentation maintainer, I want to ensure that JSON examples in documentation can be round-tripped (parsed and printed), so that the examples remain valid through transformation tools.

#### Acceptance Criteria

1. WHEN a JSON code block is validated, THE CodeBlockValidator SHALL parse the JSON into a JSON object
2. WHEN a JSON object has been parsed, THE CodeBlockValidator SHALL serialize it back to a JSON string
3. WHEN the serialized JSON is parsed again, THE JSON_Parser SHALL produce an equivalent object structure (round-trip property)
4. WHERE the round-trip result differs from the original formatting, THE CodeBlockValidator SHALL report a warning (not an error) indicating the formatting difference
5. WHEN strict formatting validation is enabled in configuration, THE CodeBlockValidator MAY report formatting inconsistencies as errors

