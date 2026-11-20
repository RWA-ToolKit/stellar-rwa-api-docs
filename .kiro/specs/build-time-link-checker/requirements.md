# Requirements Document: Build-Time Link Checker

## Introduction

The documentation site contains numerous cross-references between MDX pages. When documentation pages are renamed or deleted, inbound links silently break, creating a poor user experience and reducing documentation discoverability. A build-time link checker will validate all internal links during the Next.js build process, fail the build when broken links are detected, and provide clear error reporting to help developers fix issues quickly.

This feature integrates with the existing Next.js build pipeline and enforces link integrity before deployment to production.

## Glossary

- **LinkChecker**: The build-time tool that validates internal links in the documentation
- **Internal_Link**: A hyperlink reference from one documentation page to another page in the docs site (e.g., `/docs/getting-started`, `./compliance-guide`)
- **External_Link**: A hyperlink reference to a URL outside the documentation site (e.g., `https://github.com/RWA-ToolKit/stellar-rwa-contracts`)
- **MDX_File**: A markdown file with embedded JSX located in `docs/app/docs/**/*.mdx`
- **Link_Target**: The page or file that a link points to (e.g., `/docs/api/overview` or `page.mdx`)
- **Build_Pipeline**: The Next.js build process triggered by `npm run build` or CI/CD systems
- **CI_Failure**: When the build exits with a non-zero status code, preventing deployment
- **Resolved_Path**: The filesystem path to which a link resolves after parsing and URL normalization
- **Link_Format**: The syntax used to reference a link (e.g., Markdown `[text](/path)`, MDX JSX `href="/path"`)

## Requirements

### Requirement 1: Discover and Parse Internal Links

**User Story:** As a documentation contributor, I want all internal links in my MDX files to be discovered and parsed correctly, so that the link checker can validate them comprehensively.

#### Acceptance Criteria

1. WHEN the LinkChecker runs, THE LinkChecker SHALL scan all MDX files in `docs/app/docs/` and its subdirectories
2. WHEN an MDX file contains Markdown links in the format `[text](/path)`, THE LinkChecker SHALL extract the target path
3. WHEN an MDX file contains Markdown links in the format `[text](./relative/path)`, THE LinkChecker SHALL extract and normalize the relative path to an absolute path
4. WHEN an MDX file contains Markdown links with URL fragments (e.g., `/docs/page#section`), THE LinkChecker SHALL extract the path component and validate the page existence separately from the fragment
5. WHEN an MDX file contains JSX links with `href` attributes (e.g., `href="/docs/path"`), THE LinkChecker SHALL extract the target path
6. WHEN an MDX file contains external links (e.g., `https://github.com/...` or `mailto:...`), THE LinkChecker SHALL skip validation and not report errors for external links in the build-time check

### Requirement 2: Validate Internal Link Targets Exist

**User Story:** As a project maintainer, I want the link checker to verify that every internal link points to an existing page, so that broken links are caught before publication.

#### Acceptance Criteria

1. WHEN the LinkChecker has parsed a link to an internal path (e.g., `/docs/getting-started`), THE LinkChecker SHALL resolve the path to a corresponding MDX file in the documentation directory structure
2. WHEN a link path resolves to a directory without a trailing slash, THE LinkChecker SHALL check for a `page.mdx` file in that directory
3. WHEN a link path has a trailing slash (e.g., `/docs/api/`), THE LinkChecker SHALL check for a `page.mdx` file in the corresponding directory
4. IF a link target cannot be resolved to an existing MDX file, THEN THE LinkChecker SHALL record the broken link with the source file, line number, and target path
5. WHEN all links have been validated, THE LinkChecker SHALL output a summary of all broken links found (if any)

### Requirement 3: Integrate with the Next.js Build Pipeline

**User Story:** As a developer, I want the link checker to run automatically during the build process without requiring separate commands, so that link validation is consistently performed before every deployment.

#### Acceptance Criteria

1. WHEN `npm run build` or the Next.js build command is executed, THE LinkChecker SHALL run automatically before or after the Next.js build completes
2. THE LinkChecker SHALL be integrated as a build hook or post-build script in the Next.js build lifecycle
3. WHEN the LinkChecker runs during the build, THE LinkChecker SHALL not modify the Next.js build output or dependencies
4. WHERE the LinkChecker is triggered by a CI environment, THE LinkChecker SHALL function identically to local builds with the same validation rules and output format

### Requirement 4: Fail the Build on Broken Links

**User Story:** As a team lead, I want the build to fail when broken links are detected, so that broken documentation never reaches production.

#### Acceptance Criteria

1. WHEN the LinkChecker detects one or more broken internal links, THEN THE LinkChecker SHALL exit with a non-zero exit code
2. WHEN the LinkChecker detects one or more broken internal links, THEN THE Build_Pipeline SHALL be interrupted and the build SHALL NOT complete successfully
3. IF no broken links are found, THEN THE LinkChecker SHALL exit with exit code 0 and allow the build to proceed
4. WHEN the build fails due to broken links, THE LinkChecker SHALL display all broken links in a clear, readable format before failing

### Requirement 5: Report Broken Links with Context

**User Story:** As a developer, I want detailed error messages that tell me exactly where broken links are located and what they point to, so that I can fix them efficiently.

#### Acceptance Criteria

1. WHEN a broken link is detected, THE LinkChecker SHALL report the source file path (relative to the docs directory)
2. WHEN a broken link is detected, THE LinkChecker SHALL report the line number where the link appears in the source file
3. WHEN a broken link is detected, THE LinkChecker SHALL report the broken link target path (as written in the source)
4. WHEN a broken link is detected, THE LinkChecker SHALL report the expected file path where the target should exist
5. WHEN multiple broken links are found, THE LinkChecker SHALL group them by source file or display them in a sortable list for easy scanning
6. THE LinkChecker SHALL display the error report to stdout or stderr where it is visible in build logs

### Requirement 6: Handle Relative Links Correctly

**User Story:** As a contributor who uses relative links in documentation, I want relative links to be correctly resolved relative to the source file's location, so that my relative links work as expected.

#### Acceptance Criteria

1. WHEN a relative link is found (e.g., `./other-page` or `../sibling/page`), THE LinkChecker SHALL resolve it relative to the source MDX file's directory
2. WHEN a relative link uses `../`, THE LinkChecker SHALL traverse up the directory tree correctly
3. WHEN a relative link points outside the docs directory, THEN THE LinkChecker SHALL treat it as an external or invalid link and report an error
4. WHEN a relative link is ambiguous (e.g., `../../../page` when there are fewer directories available), THEN THE LinkChecker SHALL report an error indicating the resolved path does not exist

### Requirement 7: Provide Configuration and Control

**User Story:** As a project maintainer, I want to configure which files or directories are checked and which errors are reported, so that link checking can be customized to the project's needs.

#### Acceptance Criteria

1. WHERE a `.linkchecker.config.json` or similar configuration file exists, THE LinkChecker SHALL read configuration from that file
2. WHERE a configuration specifies ignored files or directories, THE LinkChecker SHALL skip validation for those paths
3. WHERE a configuration specifies external link validation settings, THE LinkChecker SHALL respect those settings (e.g., enable or disable external link checking)
4. WHERE no configuration file exists, THE LinkChecker SHALL use sensible defaults (validate all internal links, skip external links)

### Requirement 8: External Link Checking (Future Enhancement)

**User Story:** As a documentation maintainer, I want the option to separately check external links on a scheduled basis, so that broken external links can be detected without blocking every build.

#### Acceptance Criteria

1. WHERE external link checking is enabled in configuration, THE LinkChecker SHALL validate that external links are reachable or return HTTP 2xx/3xx responses
2. WHERE external link checking is enabled, THE LinkChecker MAY be run on a separate schedule (e.g., nightly or weekly) rather than on every build
3. WHERE external link checking is enabled, THE LinkChecker SHALL implement a timeout (e.g., 5 seconds per link) to avoid long build times
4. WHERE external link checking is enabled and a link times out or returns an error, THE LinkChecker MAY log a warning without failing the build (configurable behavior)

### Requirement 9: Handle Special Cases

**User Story:** As a contributor, I want the link checker to handle special cases correctly, so that legitimate links are not falsely reported as broken.

#### Acceptance Criteria

1. WHEN a link points to the homepage (e.g., `/`), THE LinkChecker SHALL validate that the page exists
2. WHEN a link includes query parameters (e.g., `/docs/page?tab=overview`), THE LinkChecker SHALL extract the page path and validate only the page component
3. WHEN a link is to a section anchor (e.g., `/docs/page#section`), THE LinkChecker SHALL validate the page exists but MAY skip anchor validation (see Requirement 1, criterion 4)
4. IF a page has multiple redirects, THEN THE LinkChecker MAY follow redirects or treat the original target as broken (configurable)

### Requirement 10: Support Incremental Checking (Optional)

**User Story:** As a developer working in watch mode, I want link checking to run incrementally on changed files only, so that build feedback is faster during development.

#### Acceptance Criteria

1. WHERE watch mode or incremental builds are enabled, THE LinkChecker MAY cache results from previous runs
2. WHERE a file has not changed since the last check, THE LinkChecker MAY reuse cached results for that file
3. WHERE a dependency (a page that is linked to) has changed, THE LinkChecker MAY invalidate cache entries for files that link to it
4. THE LinkChecker SHALL report accurate results even when using incremental checking; cached and fresh results MUST be identical

