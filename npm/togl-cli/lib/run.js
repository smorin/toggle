'use strict';
const { spawnSync } = require('node:child_process');

const PLATFORM_PACKAGES = {
  'linux-x64': '@smorinlabs/togl-linux-x64',
  'darwin-x64': '@smorinlabs/togl-darwin-x64',
  'darwin-arm64': '@smorinlabs/togl-darwin-arm64',
  'win32-x64': '@smorinlabs/togl-win32-x64',
};

function run(binName) {
  const key = `${process.platform}-${process.arch}`;
  const pkg = PLATFORM_PACKAGES[key];
  if (!pkg) {
    console.error(`togl-cli: unsupported platform: ${key}`);
    process.exit(1);
  }
  const exe = process.platform === 'win32' ? `${binName}.exe` : binName;
  let binPath;
  try {
    binPath = require.resolve(`${pkg}/bin/${exe}`);
  } catch {
    console.error(
      `togl-cli: platform package ${pkg} is missing.\n` +
        'It is an optionalDependency — reinstall without --no-optional / --omit=optional.'
    );
    process.exit(1);
  }
  const result = spawnSync(binPath, process.argv.slice(2), { stdio: 'inherit' });
  if (result.error) {
    console.error(`togl-cli: failed to launch ${exe}: ${result.error.message}`);
    process.exit(1);
  }
  process.exit(result.status === null ? 1 : result.status);
}

module.exports = { run };
