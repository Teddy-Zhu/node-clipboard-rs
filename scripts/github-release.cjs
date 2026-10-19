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
const api = `repos/${process.env.GITHUB_REPOSITORY}`
const gh = (args) => execFileSync('gh', args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim()
function get(endpoint) {
  try {
    return JSON.parse(gh(['api', `${api}/${endpoint}`]))
  } catch (error) {
    if (String(error.stderr).includes('(HTTP 404)')) return null
    throw error
  }
}
function checkTag() {
  // The refs API returns 404 for missing tags; the commits API can return 422 instead.
  const reference = get(`git/ref/tags/${encodeURIComponent(tag)}`)
  if (!reference) return null
  let object = reference.object
  while (object?.type === 'tag') {
    const annotated = get(`git/tags/${object.sha}`)
    assert.ok(annotated, `Cannot resolve annotated tag: ${tag}`)
    object = annotated.object
  }
  assert.equal(object?.type, 'commit', `${tag} does not point to a commit`)
  assert.equal(object.sha, release.commit, `${tag} points to another commit`)
  return object
}

function getRelease() {
  try {
    // gh also finds drafts whose pending tag has not been created yet.
    return JSON.parse(
      gh(['release', 'view', tag, '--repo', process.env.GITHUB_REPOSITORY, '--json', 'isDraft,targetCommitish,assets']),
    )
  } catch (error) {
    if (String(error.stderr).trim() === 'release not found') return null
    throw error
  }
}

const assets = [
  ...release.packages.map((entry) => entry.filename),
  ...release.binaries.map((entry) => entry.filename),
  'manifest.json',
  'SHA256SUMS',
]
const commit = checkTag()
let existing = getRelease()
if (existing?.isDraft === false) {
  assert.ok(commit, `${tag} is published but its tag is missing`)
  const published = get(`releases/tags/${tag}`)
  assert.ok(published && published.draft === false, `Published release is unavailable: ${tag}`)
  assert.deepEqual(
    new Set(published.assets.map((asset) => asset.name)),
    new Set(assets),
    'Published GitHub release assets are incomplete',
  )
  for (const asset of published.assets) {
    assert.equal(asset.state, 'uploaded', `Published asset upload is incomplete: ${asset.name}`)
    const digest = createHash('sha256')
      .update(readFileSync(join(release.directory, asset.name)))
      .digest('hex')
    assert.equal(asset.digest, `sha256:${digest}`, `Published asset contents differ: ${asset.name}`)
  }
  console.log(`GitHub Release already published with matching artifacts: ${tag}`)
} else {
  if (!existing) {
    const options = release.version.includes('-') ? ['--prerelease'] : []
    gh([
      'release',
      'create',
      tag,
      '--repo',
      process.env.GITHUB_REPOSITORY,
      '--target',
      release.commit,
      '--title',
      tag,
      '--draft',
      '--generate-notes',
      ...options,
    ])
    existing = getRelease()
  }
  assert.ok(existing?.isDraft, `Expected a draft release: ${tag}`)
  // GitHub may defer creating a draft release's tag until publication.
  if (!checkTag()) assert.equal(existing.targetCommitish, release.commit, 'Draft release targets another commit')
  gh([
    'release',
    'upload',
    tag,
    ...assets.map((filename) => join(release.directory, filename)),
    '--repo',
    process.env.GITHUB_REPOSITORY,
    '--clobber',
  ])
  const uploaded = getRelease()
  assert.deepEqual(
    new Set(uploaded.assets.map((asset) => asset.name)),
    new Set(assets),
    'GitHub release assets are incomplete',
  )
  gh(['release', 'edit', tag, '--repo', process.env.GITHUB_REPOSITORY, '--draft=false'])
  console.log(`GitHub Release published: ${tag}`)
}
