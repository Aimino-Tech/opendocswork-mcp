#!/usr/bin/env node
/**
 * install.js - Post-install binary downloader for @aimino/opendocswork-mcp
 *
 * Downloads the platform-specific binary from GitHub Releases.
 * Falls back to building from source if no prebuilt binary is available.
 */

const https = require('https');
const { createWriteStream, existsSync, mkdirSync } = require('fs');
const { platform, arch } = require('os');
const path = require('path');
const { execSync } = require('child_process');

const PACKAGE_VERSION = require('./package.json').version;
const REPO = 'Aimino-Tech/opendocswork-mcp';
const BIN_DIR = path.join(__dirname, 'bin');
const BIN_NAME = platform() === 'win32' ? 'opendocswork-mcp.exe' : 'opendocswork-mcp';
const BIN_PATH = path.join(BIN_DIR, BIN_NAME);

const TARGETS = {
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
  'win32-x64': 'x86_64-pc-windows-msvc',
};

function getTarget() {
  const key = `${platform()}-${arch()}`;
  return TARGETS[key] || null;
}

function download(url, dest) {
  return new Promise((resolve, reject) => {
    const file = createWriteStream(dest);
    https.get(url, (response) => {
      if (response.statusCode !== 200) {
        reject(new Error(`Download failed: ${response.statusCode}`));
        return;
      }
      response.pipe(file);
      file.on('finish', () => {
        file.close();
        if (platform() !== 'win32') {
          execSync(`chmod +x "${dest}"`);
        }
        resolve();
      });
    }).on('error', reject);
  });
}

async function main() {
  if (!existsSync(BIN_DIR)) mkdirSync(BIN_DIR, { recursive: true });

  const target = getTarget();
  if (!target) {
    console.error(`Unsupported platform: ${platform()}-${arch()}`);
    console.error('Falling back: binary must be built from source');
    process.exit(1);
  }

  const url = `https://github.com/${REPO}/releases/download/v${PACKAGE_VERSION}/opendocswork-mcp-${target}.tar.gz`;

  console.log(`Downloading ${BIN_NAME} for ${target}...`);
  try {
    await download(url, BIN_PATH);
    console.log(`Installed ${BIN_PATH}`);
  } catch (e) {
    console.error(`Failed to download binary: ${e.message}`);
    process.exit(1);
  }
}

main();
