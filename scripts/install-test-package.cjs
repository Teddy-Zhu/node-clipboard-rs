const assert = require('node:assert/strict')
const { execFileSync } = require('node:child_process')
const { appendFileSync, mkdtempSync, readFileSync, writeFileSync } = require('node:fs')
const { createRequire } = require('node:module')
const { tmpdir } = require('node:os')
const { dirname, join, resolve } = require('node:path')
const { loadRelease } = require('./release-manifest.cjs')

const release = loadRelease()
const abi = process.platform === 'win32' ? '-msvc' : process.platform === 'linux' ? '-gnu' : ''
const nativeName = `${release.name}-${process.platform}-${process.arch}${abi}`
const selected = [release.name, nativeName].map((name) => {
  const entry = release.packages.find((item) => item.name === name)
  assert.ok(entry, `No prebuilt package for ${process.platform}/${process.arch}`)
  return resolve(release.directory, entry.filename)
})
const directory = mkdtempSync(join(tmpdir(), 'clipboard-package-'))
writeFileSync(join(directory, 'package.json'), '{"name":"clipboard-install-test","version":"1.0.0","private":true}\n')
// setup-node installs the official Node distribution, which includes npm here.
const npmCli = resolve(
  dirname(process.execPath),
  process.platform === 'win32' ? 'node_modules/npm/bin/npm-cli.js' : '../lib/node_modules/npm/bin/npm-cli.js',
)
execFileSync(
  process.execPath,
  [
    npmCli,
    'install',
    '--offline',
    '--ignore-scripts',
    '--no-audit',
    '--no-fund',
    '--cache',
    join(directory, 'cache'),
    ...selected,
  ],
  {
    cwd: directory,
    stdio: 'inherit',
  },
)
const installedRequire = createRequire(join(directory, 'package.json'))
const bindingPath = installedRequire.resolve(release.name)
const packageRequire = createRequire(bindingPath)
assert.equal(packageRequire(`${nativeName}/package.json`).version, release.version)
assert.equal(JSON.parse(readFileSync(join(dirname(bindingPath), 'package.json'), 'utf8')).version, release.version)
process.env.NAPI_RS_ENFORCE_VERSION_CHECK = '1'
const binding = installedRequire(release.name)
assert.equal(typeof binding.ClipboardManager, 'function')
assert.ok(require.cache[packageRequire.resolve(nativeName)], 'The installed native package was not loaded')
if (process.env.GITHUB_ENV)
  appendFileSync(process.env.GITHUB_ENV, `CLIPBOARD_TEST_PACKAGE=${bindingPath}\nNAPI_RS_ENFORCE_VERSION_CHECK=1\n`)
console.log(`Installed and loaded ${nativeName}@${release.version}`)
console.log(`CLIPBOARD_TEST_PACKAGE=${bindingPath}`)
