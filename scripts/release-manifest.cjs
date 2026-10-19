const assert = require('node:assert/strict')
const { createHash } = require('node:crypto')
const { readFileSync } = require('node:fs')
const { basename, resolve } = require('node:path')
const pkg = require('../package.json')

function loadRelease(directory = 'packages') {
  const root = resolve(directory)
  const manifest = JSON.parse(readFileSync(resolve(root, 'manifest.json'), 'utf8'))
  assert.equal(manifest.name, pkg.name)
  assert.equal(manifest.version, pkg.version)
  assert.equal(manifest.packages.length, pkg.napi.targets.length + 1)
  assert.equal(new Set(manifest.packages.map((entry) => entry.name)).size, manifest.packages.length)
  assert.equal(manifest.binaries.length, pkg.napi.targets.length)
  assert.ok(manifest.packages.some((entry) => entry.name === pkg.name))
  for (const entry of manifest.packages) {
    assert.equal(entry.version, pkg.version)
    assert.ok(entry.name === pkg.name || entry.name.startsWith(`${pkg.name}-`))
    assert.equal(entry.filename, basename(entry.filename))
    const integrity = `sha512-${createHash('sha512')
      .update(readFileSync(resolve(root, entry.filename)))
      .digest('base64')}`
    assert.equal(integrity, entry.integrity, `Tarball integrity mismatch: ${entry.filename}`)
  }
  for (const entry of manifest.binaries) {
    assert.equal(entry.filename, basename(entry.filename))
    const digest = createHash('sha256')
      .update(readFileSync(resolve(root, entry.filename)))
      .digest('hex')
    assert.equal(digest, entry.sha256, `Binary checksum mismatch: ${entry.filename}`)
  }
  return { ...manifest, directory: root }
}

module.exports = { loadRelease }
