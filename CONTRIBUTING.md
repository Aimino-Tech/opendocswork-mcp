# Contributing to office-oxide-mcp

Thank you for your interest in contributing! office-oxide-mcp is an open source project and we welcome contributions of all kinds — code, documentation, skills, issues, and community support.

---

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How to Contribute](#how-to-contribute)
- [Contributing Skills](#contributing-skills)
- [Contributing Code](#contributing-code)
- [Contributing Documentation](#contributing-documentation)
- [Reporting Bugs](#reporting-bugs)
- [Feature Requests](#feature-requests)
- [Development Setup](#development-setup)
- [Pull Request Process](#pull-request-process)
- [Community](#community)

---

## Code of Conduct

This project follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). Be respectful, inclusive, and professional. Harassment of any kind will not be tolerated.

---

## How to Contribute

### I want to...

| Goal | Where to Start |
|---|---|
| Create a Skill | [Skills Guide](#contributing-skills) |
| Report a bug | [Bug Report](#reporting-bugs) |
| Suggest a feature | [Feature Request](#feature-requests) |
| Fix a bug | Check [Issues](https://github.com/Aimino-Tech/office-oxide-mcp/issues) labeled `good first issue` or `bug` |
| Improve docs | [Documentation Guide](#contributing-documentation) |
| Write code | [Code Contribution Guide](#contributing-code) |
| Ask a question | Open a [Discussion](https://github.com/Aimino-Tech/office-oxide-mcp/discussions) |

---

## Contributing Skills

Skills are the heart of the office-oxide-mcp ecosystem. A Skill is a versioned package containing:

- **Template**: A professionally designed `.docx` / `.xlsx` / `.pptx` with `{placeholders}`
- **Tool Composition**: Which MCP tools to call in which order
- **Formatting Rules**: Brand colors, fonts, margins, styles
- **Validation**: Quality checks to run after generation

### Skill Format (YAML)

```yaml
name: sales-report
version: "1.0.0"
description: Generate a monthly sales report with charts
format: xlsx
template: templates/sales-report.xlsx
steps:
  - tool: office_write_range
    params: { sheet: "Data", range: "A1:D100", data: "${data}" }
  - tool: office_create_chart
    params: { sheet: "Data", type: "bar", range: "A1:D100" }
validation:
  - rule: no_empty_placeholders
  - rule: brand_colors_match
    params: { palette: ["#1a1a2e", "#f59e0b"] }
```

### Submitting a Skill

1. Create your Skill as a `.yaml` file with template assets
2. Submit via pull request to the `skills/` directory
3. Include a README with usage examples and screenshots

---

## Contributing Code

### Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/office-oxide-mcp.git`
3. Create a feature branch: `git checkout -b feat/your-feature-name`
4. Follow the [Development Guide](docs/development.md) for setup

### Pre-commit Hooks

Install pre-commit to automatically run clippy and formatter before every commit:

```bash
make setup-hooks     # Install pre-commit + configure hooks
```

This runs `cargo clippy --fix` and `cargo fmt` on staged Rust files automatically. If the hooks fail, your commit is blocked until the issues are resolved.

### Quick Fix Commands

```bash
make fix             # Auto-fix: clippy --fix + cargo fmt (all targets)
make fix-ci          # Same but with -- -D warnings (stricter)
make lint            # Check-only: clippy + fmt (alias for make check)
make check           # Same as make lint
```

### Code Style

- Run `cargo fmt` before committing (or use `make fix` / pre-commit hooks)
- Run `cargo clippy -- -D warnings` — zero warnings required
- Run `cargo test` — all tests must pass
- Follow existing patterns (file structure, error handling, naming)

### Commit Messages

```
feat: add office_create_chart tool for bar/line/pie charts
fix: handle empty sheets in office_read
docs: update API.md with new tool parameters
```

Use conventional commit prefixes: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`.

### Pull Request Checklist

- [ ] Code compiles with `cargo build`
- [ ] All tests pass with `cargo test`
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Formatted with `cargo fmt`
- [ ] New tools include E2E tests with real binary
- [ ] API.md updated for new/changed tools
- [ ] CHANGELOG.md updated

---

## Contributing Documentation

Good documentation is as important as good code. We welcome:

- Fixing typos or unclear explanations
- Adding examples and use cases
- Translating docs to other languages
- Creating tutorials or video guides

Documentation lives in the `docs/` directory and uses Markdown.

---

## Reporting Bugs

Open a [Bug Report](https://github.com/Aimino-Tech/office-oxide-mcp/issues/new?template=bug_report.md) with:

- **Summary**: Clear description of the issue
- **Steps to reproduce**: Minimal, complete, verifiable example
- **Expected behavior**: What should happen
- **Actual behavior**: What actually happens
- **Environment**: OS, Rust version, MCP client, file format
- **Logs**: Error output from the server (stderr)

---

## Feature Requests

Open a [Feature Request](https://github.com/Aimino-Tech/office-oxide-mcp/issues/new?template=feature_request.md) with:

- **Problem**: What problem are you trying to solve?
- **Solution**: What would you like the tool to do?
- **Alternatives**: What have you tried?
- **Context**: Any additional context, screenshots, or examples

---

## Development Setup

See [docs/development.md](docs/development.md) for full setup instructions.

Quick start:

```bash
git clone https://github.com/Aimino-Tech/office-oxide-mcp.git
cd office-oxide-mcp
cargo build
cargo test
```

---

## Pull Request Process

1. **Small PRs preferred** — One feature or fix per PR. Large changes should be discussed first.
2. **Draft PRs welcome** — Open early for feedback on approach.
3. **CI must pass** — Build, test, clippy, format check all green.
4. **Review required** — At least one maintainer review before merge.
5. **Squash merge** — Commits will be squashed into a single commit.

---

## Community

- **GitHub Discussions** — Questions, ideas, show-and-tell: https://github.com/Aimino-Tech/office-oxide-mcp/discussions
- **Discord** — Real-time chat: https://discord.gg/bPJ7XqYsTC
- **Skills Marketplace** — Share and discover community skills: https://github.com/Aimino-Tech/office-oxide-skills
- **Office Hours** — Monthly video calls (schedule TBD)
- **COMMUNITY.md** — Full community guide with Discord channels, skills marketplace details, and more

---

## License

By contributing, you agree that your contributions will be licensed under the Apache 2.0 / MIT license as specified in the repository.
