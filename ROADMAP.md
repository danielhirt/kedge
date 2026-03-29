# Roadmap

Planned work for kedge, roughly in priority order.

## In Progress

### Docs site readability
Improve fonts, spacing, and typography on the mdBook docs site. The current styling needs refinement for legibility at different screen sizes.

## Planned

### Logging and telemetry
Add structured logging throughout the pipeline (detection, triage, remediation) and optional telemetry for tracking drift trends over time. Design decisions needed around log levels, output format, and opt-in metrics collection.

### Regex and glob patterns in anchor frontmatter
Allow anchors to target multiple files via glob patterns (e.g., `src/auth/**/*.java`) or regex matches instead of requiring one anchor per file. Reduces frontmatter boilerplate for docs that track an entire module or directory.
