# Changelog

## [0.1.0] - 2026-05-21

### Added

- Edge/Serverless deployment infrastructure:
  - `.cargo/config.toml` — musl target config (`rust-lld` linker, `+crt-static`), release LTO profile
  - `Makefile` — `build-lambda`, `build-docker`, `package-lambda`, `test-lambda`, `check` targets
  - `Dockerfile` — multi-stage build: `rust:1.79-alpine3.20` builder → `scratch` runtime (<5MB)
  - `bootstrap` — AWS Lambda custom runtime entry point
  - `.github/workflows/release.yml` — added `build-lambda` (musl→Lambda zip) + `build-docker` (GHCR push) jobs
  - `tests/deployment_test.rs` — binary size, Docker image, Lambda zip, stdio smoke tests
- Skills System Framework (`src/skills/`):
  - `SkillDefinition`, `SkillRegistry`, `SkillRunner`, `SkillValidator` with 5 validation rules
  - 5 MCP tools: `skill_run`, `skill_list`, `skill_validate`, `skill_register`, `skill_remove`
  - 3 built-in skills: `excel.table`, `word.report`, `ppt.deck`
  - YAML format with compact/full style support for tools and placeholders
  - Filesystem persistence (skills persist in `skills/` directory across restarts)
  - Fix loop: validation report includes `fix_tool`/`fix_args` for empty placeholders
  - 23 E2E tests
- Word Document Reader (`src/readers/`):
  - Semantic DOCX extraction (paragraphs, tables, images, headers, footers, comments)
  - 3 output formats: structured JSON, Markdown, semantic chunks
  - Reading pipeline with section detection and image embedding
- Coherence Engine (`src/coherence/`):
  - Entity-aware DAG for cross-document reference tracking
  - Entity graph with SHA-256 content hashing
  - BFS propagation, stale detection, integrity verification
  - 3 MCP tools: `office_propagate_edit`, `office_check_consistency`, `office_get_entity_graph`
- CI/CD pipeline (GitHub Actions) for Linux, macOS, Windows
- Cross-platform build targets: x86_64 + aarch64 for Linux, macOS, Windows
- E2E test framework with real MCP server process spawning
- Documentation: README, Getting Started, Architecture, Development, API Reference
- Apache 2.0 / MIT dual license
- Issue templates: Bug Report, Feature Request, Skill Submission
- Security policy (SECURITY.md)
- Contributing guide (CONTRIBUTING.md)
- Code of Conduct (CODE_OF_CONDUCT.md)
- Launch checklist, HN/Reddit post templates
- MCP directory submission guides
- Dockerfile for containerized deployment
- Smithery configuration (`smithery.yaml`)
- Launch scripts for testing

---

## [0.1.0] — 2026-05-20

### Added

- Project scaffold and initial commit
- Rust MCP server with rmcp stdio transport
- 5 foundation tools:
  - `list_formats` — format enumeration
  - `get_document_info` — metadata extraction
  - `office_read` — document reading (JSON/Markdown/Chunks)
  - `increment` — counter demo tool
  - `get_value` — counter demo tool
- `schemars` JSON Schema generation for all tool parameters
- `tokio` async runtime with multi-threaded I/O
- `tracing` / `tracing-subscriber` for structured logging
- Binary size: ~4MB static release binary

[Unreleased]: https://github.com/Aimino-Tech/office-oxide-mcp/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Aimino-Tech/office-oxide-mcp/releases/tag/v0.1.0
