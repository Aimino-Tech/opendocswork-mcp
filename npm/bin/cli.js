#!/usr/bin/env node
const { execSync } = require('child_process');
const path = require('path');

// Resolve the binary next to this script
const binName = process.platform === 'win32' ? 'opendocswork-mcp.exe' : 'opendocswork-mcp';
const binPath = path.join(__dirname, '..', 'bin', binName);

try {
  execSync(binPath, { stdio: 'inherit' });
} catch (e) {
  process.exit(e.status || 1);
}
