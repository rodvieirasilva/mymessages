import { execSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { defineConfig } from 'vite';

function getGitSha(): string {
  try {
    return execSync('git rev-parse --short HEAD', { encoding: 'utf-8' }).trim();
  } catch {
    return 'dev';
  }
}

function getAppVersion(): string {
  try {
    const pkg = JSON.parse(readFileSync('./package.json', 'utf-8'));
    return pkg.version || '0.2.0';
  } catch {
    return '0.2.0';
  }
}

const buildTime = new Date().toISOString().replace('T', ' ').slice(0, 16) + ' UTC';

export default defineConfig({
  define: {
    __APP_VERSION__: JSON.stringify(getAppVersion()),
    __BUILD_TIME__: JSON.stringify(buildTime),
    __GIT_SHA__: JSON.stringify(getGitSha()),
  },
});
