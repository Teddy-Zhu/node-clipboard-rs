const assert = require('node:assert/strict')
const { execFileSync } = require('node:child_process')
const { resolve } = require('node:path')
const { loadRelease } = require('./release-manifest.cjs')

const release = loadRelease()
const mode = process.argv[2]
assert.ok(mode === 'platforms' || mode === 'main', 'Expected platforms or main')
const entries = release.packages.filter((entry) => (entry.name === release.name) === (mode === 'main'))

function npm(args) {
  return execFileSync('npm', [...args, '--registry=https://registry.npmjs.org'], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trim()
}

function publishedIntegrity(entry) {
  try {
    return JSON.parse(npm(['view', `${entry.name}@${entry.version}`, 'dist.integrity', '--json']))
  } catch (error) {
    let code
    try {
      code = JSON.parse(String(error.stdout)).error?.code
    } catch {
      /* Non-JSON errors retain their original diagnostics. */
    }
    if (code === 'E404') return null
    throw error
  }
}

async function publish() {
  for (const entry of entries) {
    const existing = publishedIntegrity(entry)
    if (existing !== null) {
      assert.equal(existing, entry.integrity, `Published contents differ: ${entry.name}@${entry.version}`)
      console.log(`Already published with matching integrity: ${entry.name}@${entry.version}`)
      continue
    }
    console.log(
      npm([
        'publish',
        resolve(release.directory, entry.filename),
        '--ignore-scripts',
        '--access',
        'public',
        '--tag',
        release.version.includes('-') ? 'next' : 'latest',
      ]),
    )
    let available = false
    for (let attempt = 0; attempt < 12; attempt++) {
      const integrity = publishedIntegrity(entry)
      if (integrity !== null) {
        assert.equal(integrity, entry.integrity, `Registry integrity mismatch: ${entry.name}`)
        available = true
        break
      }
      if (attempt < 11) await new Promise((resolve) => setTimeout(resolve, 5000))
    }
    assert.ok(available, `Published package is not visible: ${entry.name}@${entry.version}`)
  }
}

publish().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
