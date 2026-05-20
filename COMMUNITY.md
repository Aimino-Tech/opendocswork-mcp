# office-oxide-mcp Community

Welcome to the office-oxide-mcp community! We're building the fastest open-source Office document processing engine together.

## Quick Links

- **Discord**: [Join our Discord server](https://discord.gg/bPJ7XqYsTC) — Community chat, help, and showcases
- **GitHub Discussions**: [Start a discussion](https://github.com/Aimino-Tech/office-oxide-mcp/discussions) — Q&A, ideas, show and tell
- **GitHub Issues**: [Report a bug](https://github.com/Aimino-Tech/office-oxide-mcp/issues) or [request a feature](https://github.com/Aimino-Tech/office-oxide-mcp/issues/new)
- **X / Twitter**: Follow [@AiminoTech](https://twitter.com/AiminoTech) for updates

## How to Get Involved

| Activity | Where | Description |
|----------|-------|-------------|
| Ask questions | [Discord #help](https://discord.gg/bPJ7XqYsTC) or [GitHub Discussions Q&A](https://github.com/Aimino-Tech/office-oxide-mcp/discussions/categories/q-a) | Get help with installation, usage, and troubleshooting |
| Share your work | [Discord #showcase](https://discord.gg/bPJ7XqYsTC) or [GitHub Discussions Show and Tell](https://github.com/Aimino-Tech/office-oxide-mcp/discussions/categories/show-and-tell) | Show what you've built with office-oxide-mcp |
| Contribute code | [CONTRIBUTING.md](CONTRIBUTING.md) | PRs welcome! See our contribution guide |
| Create skills | [Skills Marketplace](https://github.com/Aimino-Tech/office-oxide-skills) | Build and share reusable skill templates |
| Report bugs | [GitHub Issues](https://github.com/Aimino-Tech/office-oxide-mcp/issues) | Found a bug? Let us know |
| Feature ideas | [GitHub Issues](https://github.com/Aimino-Tech/office-oxide-mcp/issues) | Request new tools, formats, or capabilities |

## Discord Channels

- `#welcome` — Rules, install guide, getting started
- `#general` — General discussion and announcements
- `#help` — User support and troubleshooting
- `#showcase` — Share what you've built
- `#contributing` — Development discussion and PR review
- `#skill-development` — Custom skill creation and sharing
- `#releases` — Version announcements (read-only)

## Skills Marketplace

The [office-oxide-skills](https://github.com/Aimino-Tech/office-oxide-skills) repository is our community skills marketplace. Skills are reusable YAML templates that define document generation workflows (invoices, reports, presentations, etc.).

### How to Publish a Skill

1. Create a YAML skill definition file following the [skill schema](skills/)
2. Test it locally with `skill_run`
3. Submit a PR to [Aimino-Tech/office-oxide-skills](https://github.com/Aimino-Tech/office-oxide-skills)
4. Once approved, your skill is available to the entire community

### How to Use Community Skills

```bash
# Clone the skills marketplace
git clone https://github.com/Aimino-Tech/office-oxide-skills.git

# Point office-oxide-mcp to the skills directory
office-oxide-mcp --skills-dir ./office-oxide-skills
```

## GitHub Discussions Categories

- **Q&A** — Questions about installation, configuration, and usage
- **Show and Tell** — Share your projects and use cases
- **Ideas** — Feature requests and improvement suggestions
- **General** — Anything else related to office-oxide-mcp

## Code of Conduct

This community follows the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). Be respectful, inclusive, and professional.

## License

office-oxide-mcp is dual-licensed under [Apache 2.0](LICENSE-APACHE) and [MIT](LICENSE-MIT).
