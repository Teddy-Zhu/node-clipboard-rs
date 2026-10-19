const assert = require('node:assert/strict')
const { execFileSync } = require('node:child_process')
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
  const commit = get(`commits/${tag}`)
  if (commit) assert.equal(commit.sha, release.commit, `${tag} points to another commit`)
  return commit
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

checkTag()
let existing = getRelease()
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
assert.ok(existing?.isDraft, `${tag} is already published; published assets will not be replaced`)
// GitHub may defer creating a draft release's tag until publication.
if (!checkTag()) assert.equal(existing.targetCommitish, release.commit, 'Draft release targets another commit')
const assets = [
  ...release.packages.map((entry) => entry.filename),
  ...release.binaries.map((entry) => entry.filename),
  'manifest.json',
  'SHA256SUMS',
]
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
