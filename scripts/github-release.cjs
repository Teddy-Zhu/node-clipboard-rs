const assert = require('node:assert/strict')
const { execFileSync } = require('node:child_process')
const { createHash } = require('node:crypto')
const { readFileSync } = require('node:fs')
const { join } = require('node:path')
const { loadRelease } = require('./release-manifest.cjs')

const release = loadRelease()
assert.ok(process.env.GITHUB_REPOSITORY && process.env.GITHUB_SHA, 'Run this script from the release workflow')
assert.equal(release.commit, process.env.GITHUB_SHA, 'Release artifacts belong to a different commit')
const tag = `v${release.version}`
const api = `${process.env.GITHUB_API_URL || 'https://api.github.com'}/repos/${process.env.GITHUB_REPOSITORY}`
const token = process.env.GH_TOKEN || process.env.GITHUB_TOKEN
assert.ok(token, 'GH_TOKEN or GITHUB_TOKEN is required')
const gh = (args) => execFileSync('gh', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim()
async function get(endpoint, allowMissing = false) {
  const response = await fetch(`${api}/${endpoint}`, {
    headers: {
      Accept: 'application/vnd.github+json',
      Authorization: `Bearer ${token}`,
      'X-GitHub-Api-Version': '2022-11-28',
    },
    signal: AbortSignal.timeout(30_000),
  })
  if (allowMissing && response.status === 404) {
    await response.body?.cancel()
    return null
  }
  if (!response.ok) throw new Error(`GitHub API ${endpoint}: HTTP ${response.status}\n${await response.text()}`)
  return response.json()
}
async function checkTag() {
  // The refs API returns 404 for missing tags; the commits API can return 422 instead.
  const reference = await get(`git/ref/tags/${encodeURIComponent(tag)}`, true)
  if (!reference) return null
  let object = reference.object
  while (object?.type === 'tag') {
    const annotated = await get(`git/tags/${object.sha}`)
    object = annotated.object
  }
  assert.equal(object?.type, 'commit', `${tag} does not point to a commit`)
  assert.equal(object.sha, release.commit, `${tag} points to another commit`)
  return object
}

async function getRelease() {
  const published = await get(`releases/tags/${encodeURIComponent(tag)}`, true)
  if (published) return published
  // The tag endpoint omits drafts. List releases with authentication to find pending tags.
  // https://github.com/softprops/action-gh-release/blob/v3.0.3/src/github.ts
  for (let page = 1; ; page++) {
    const releases = await get(`releases?per_page=100&page=${page}`)
    assert.ok(Array.isArray(releases), 'Expected a GitHub releases list')
    const draft = releases.find((entry) => entry.tag_name === tag && entry.draft)
    if (draft) return draft
    if (releases.length < 100) return null
  }
}

async function main() {
  const assets = [
    ...release.packages.map((entry) => entry.filename),
    ...release.binaries.map((entry) => entry.filename),
    'manifest.json',
    'SHA256SUMS',
  ]
  const commit = await checkTag()
  let existing = await getRelease()
  if (existing?.draft === false) {
    assert.ok(commit, `${tag} is published but its tag is missing`)
    assert.deepEqual(
      new Set(existing.assets.map((asset) => asset.name)),
      new Set(assets),
      'Published GitHub release assets are incomplete',
    )
    for (const asset of existing.assets) {
      assert.equal(asset.state, 'uploaded', `Published asset upload is incomplete: ${asset.name}`)
      const digest = createHash('sha256')
        .update(readFileSync(join(release.directory, asset.name)))
        .digest('hex')
      assert.equal(asset.digest, `sha256:${digest}`, `Published asset contents differ: ${asset.name}`)
    }
    console.log(`GitHub Release already published with matching artifacts: ${tag}`)
  } else {
    if (!existing) {
      // Use the creation response directly; a new draft need not be visible in listings yet.
      existing = JSON.parse(
        gh([
          'api',
          `${api}/releases`,
          '--method',
          'POST',
          '-f',
          `tag_name=${tag}`,
          '-f',
          `target_commitish=${release.commit}`,
          '-f',
          `name=${tag}`,
          '-F',
          'draft=true',
          '-F',
          'generate_release_notes=true',
          '-F',
          `prerelease=${release.version.includes('-')}`,
        ]),
      )
    }
    assert.ok(existing?.draft, `Expected a draft release: ${tag}`)
    // GitHub may defer creating a draft release's tag until publication.
    if (!(await checkTag()))
      assert.equal(existing.target_commitish, release.commit, 'Draft release targets another commit')
    gh([
      'release',
      'upload',
      tag,
      ...assets.map((filename) => join(release.directory, filename)),
      '--repo',
      process.env.GITHUB_REPOSITORY,
      '--clobber',
    ])
    const uploaded = await get(`releases/${existing.id}`)
    assert.ok(uploaded.draft, `Expected a draft release before publishing: ${tag}`)
    assert.deepEqual(
      new Set(uploaded.assets.map((asset) => asset.name)),
      new Set(assets),
      'GitHub release assets are incomplete',
    )
    gh(['api', `${api}/releases/${existing.id}`, '--method', 'PATCH', '-F', 'draft=false'])
    console.log(`GitHub Release published: ${tag}`)
  }
}

main().catch((error) => {
  console.error(error)
  process.exitCode = 1
})
