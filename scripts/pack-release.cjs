const assert = require('node:assert/strict')
const { execFileSync } = require('node:child_process')
const { createHash } = require('node:crypto')
const { copyFileSync, mkdirSync, readFileSync, readdirSync, writeFileSync } = require('node:fs')
const { join, resolve } = require('node:path')
const pkg = require('../package.json')
const { loadRelease } = require('./release-manifest.cjs')

const directory = resolve('packages')
mkdirSync(directory, { recursive: true })
const manifest = {
  name: pkg.name,
  version: pkg.version,
  commit: process.env.GITHUB_SHA || null,
  packages: [],
  binaries: [],
}

for (const platform of readdirSync('npm').sort()) {
  const source = join('npm', platform)
  const metadata = JSON.parse(readFileSync(join(source, 'package.json'), 'utf8'))
  copyFileSync(join(source, metadata.main), join(directory, metadata.main))
  manifest.binaries.push({
    filename: metadata.main,
    sha256: createHash('sha256')
      .update(readFileSync(join(source, metadata.main)))
      .digest('hex'),
  })
  pack(source, metadata.main)
}
pack('.')
writeFileSync(join(directory, 'manifest.json'), `${JSON.stringify(manifest, null, 2)}\n`)
loadRelease(directory)
const checksums = [
  ...manifest.packages.map((entry) => entry.filename),
  ...manifest.binaries.map((entry) => entry.filename),
  'manifest.json',
]
  .sort()
  .map(
    (filename) =>
      `${createHash('sha256')
        .update(readFileSync(join(directory, filename)))
        .digest('hex')}  ${filename}`,
  )
writeFileSync(join(directory, 'SHA256SUMS'), `${checksums.join('\n')}\n`)
console.log(`Packed ${manifest.packages.length} npm packages and ${manifest.binaries.length} native binaries`)

function pack(source, binary) {
  const metadata = JSON.parse(readFileSync(join(source, 'package.json'), 'utf8'))
  const output = JSON.parse(
    execFileSync('npm', ['pack', '--ignore-scripts', '--json', '--pack-destination', directory], {
      cwd: source,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'inherit'],
    }),
  )
  // npm 11 returns an array; npm 12 returns an object keyed by package name.
  const results = Array.isArray(output) ? output : Object.values(output ?? {})
  assert.equal(results.length, 1, `Expected one npm pack result: ${metadata.name}`)
  const [result] = results
  assert.equal(result?.name, metadata.name, `Packed package name mismatch: ${metadata.name}`)
  assert.equal(result.version, metadata.version, `Packed package version mismatch: ${metadata.name}`)
  assert.ok(Array.isArray(result.files), `Missing npm pack file list: ${metadata.name}`)
  const files = result.files.map((file) => file.path)
  if (binary) {
    assert.deepEqual(
      files.filter((file) => file.endsWith('.node')),
      [binary],
    )
  } else {
    for (const required of ['index.js', 'index.d.ts', 'README.md', 'README.zh-CN.md', 'LICENSE'])
      assert.ok(files.includes(required), `Missing packaged file: ${required}`)
    assert.ok(!files.some((file) => file.endsWith('.node')), 'The main package must use platform dependencies')
  }
  manifest.packages.push({
    name: result.name,
    version: result.version,
    filename: result.filename,
    integrity: result.integrity,
  })
}
