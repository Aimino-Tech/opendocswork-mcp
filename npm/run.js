#!/usr/bin/env node
/**
 * @aimino/opendocswork-mcp - Binary downloader for Rust MCP server
 *
 * Downloads the appropriate binary for the current platform on postinstall
 * and runs it as a CLI.
 */
const { execSync } = require('child_process');
const path = require('path');

const binName = process.platform === 'win32' ? 'opendocswork-mcp.exe' : 'opendocswork-mcp';
const binPath = path.join(__dirname, 'bin', binName);

try {
  execSync(binPath, { stdio: 'inherit' });
} catch (e) {
  process.exit(e.status || 1);
}
