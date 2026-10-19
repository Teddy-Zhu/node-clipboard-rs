const assert = require('node:assert/strict')
const { execFileSync } = require('node:child_process')
const { resolve } = require('node:path')
const { loadRelease } = require('./release-manifest.cjs')

const registry = 'https://registry.npmjs.org'
const release = loadRelease()
const mode = process.argv[2]
assert.ok(mode === 'platforms' || mode === 'main', 'Expected platforms or main')
const entries = release.packages.filter((entry) => (entry.name === release.name) === (mode === 'main'))

function npm(args) {
  return execFileSync('npm', [...args, `--registry=${registry}`], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  }).trim()
}

async function publishedIntegrity(entry) {
  const spec = `${entry.name}@${entry.version}`
  const url = new URL(`${registry}/${encodeURIComponent(entry.name)}/${encodeURIComponent(entry.version)}`)
  // Avoid stale package metadata and cached 404s immediately after publishing.
  url.searchParams.set('cacheBust', Date.now().toString())
  const response = await fetch(url, {
    headers: { accept: 'application/json', 'cache-control': 'no-cache' },
    cache: 'no-store',
    signal: AbortSignal.timeout(15000),
  })
  if (!response.ok) {
    await response.body?.cancel()
    if (response.status === 404) return null
    throw new Error(`Registry lookup failed for ${spec}: HTTP ${response.status}`)
  }
  const metadata = await response.json()
  assert.equal(metadata.name, entry.name, `Registry package name mismatch: ${spec}`)
  assert.equal(metadata.version, entry.version, `Registry version mismatch: ${spec}`)
  assert.equal(typeof metadata.dist?.integrity, 'string', `Missing registry integrity: ${spec}`)
  return metadata.dist.integrity
}

async function alreadyPublished(entry) {
  const spec = `${entry.name}@${entry.version}`
  const integrity = await publishedIntegrity(entry)
  if (integrity === null) return false
  assert.equal(
    integrity,
    entry.integrity,
    `Published contents differ: ${spec}. Use a new version for changed contents.`,
  )
  console.log(`Already published with matching integrity: ${spec}`)
  return true
}

async function publish() {
  for (const entry of entries) {
    if (await alreadyPublished(entry)) continue
    // A successful publish is authoritative; registry reads can lag behind writes.
    try {
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
    } catch (publishError) {
      // Another run, or a lost publish response, may leave this version already published.
      try {
        if (await alreadyPublished(entry)) {
          console.warn(`npm publish returned an error; matching published contents were verified: ${entry.name}`)
          continue
        }
      } catch (registryError) {
        throw new AggregateError(
          [publishError, registryError],
          `Failed to publish or verify ${entry.name}@${entry.version}`,
        )
      }
      throw publishError
    }
  }
}

publish().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
