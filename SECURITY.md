# Security Policy

## Supported Versions

| Version | Supported |
|---|---|
| 0.1.x | ✅ Active development |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security issue in office-oxide-mcp, please do NOT file a public GitHub issue.

**Instead, report via:**

1. **GitHub Security Advisory:** Use the "Report a Vulnerability" button at:
   https://github.com/Aimino-Tech/office-oxide-mcp/security/advisories

2. **Email:** Send details to security@aimino.tech

Please include:
- Type of vulnerability (e.g., RCE, path traversal, XXE)
- Steps to reproduce (minimal example or file)
- Affected versions
- Potential impact

We will:
- Acknowledge receipt within 48 hours
- Provide a timeline for fix and disclosure
- Credit reporters in release notes (unless anonymity requested)

## Scope

The following are in scope:
- The `office-oxide-mcp` binary and its source code
- MCP tool input validation (malicious file paths, format injections)
- XML external entity (XXE) processing
- ZIP path traversal (malicious OOXML archives)

## Out of Scope

The following are out of scope:
- MCP client software (Claude Desktop, Cursor, VS Code, etc.)
- Operating system or hardware vulnerabilities
- Third-party Rust crate vulnerabilities (report to respective maintainers)

## Disclosure Policy

We follow coordinated disclosure:
1. Reporter submits vulnerability
2. We confirm and develop fix (typically within 14 days)
3. Fix released in new version with advisory
4. Public disclosure after 30 days or when fix is available

## Preferred Languages

English preferred. Japanese, Chinese, and Korean also accepted.
