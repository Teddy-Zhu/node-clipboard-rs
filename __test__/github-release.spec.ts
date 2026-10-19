import test from 'ava'
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { readFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { join } from 'node:path'
import { runInNewContext } from 'node:vm'

const require = createRequire(import.meta.url)
const source = readFileSync(new URL('../scripts/github-release.cjs', import.meta.url), 'utf8')
const commit = 'a'.repeat(40)
const tag = 'v0.0.12'
const filenames = ['clipboard.tgz', 'clipboard.node', 'manifest.json', 'SHA256SUMS']
const contents = (filename: string) => Buffer.from(join('/release', filename))
const assets = filenames.map((name) => ({
  name,
  state: 'uploaded',
  digest: `sha256:${createHash('sha256').update(contents(name)).digest('hex')}`,
}))

interface Options {
  draft?: boolean
  tag?: 'annotated' | 'wrong'
  secondPage?: boolean
  wrongDraftTarget?: boolean
  changedAsset?: boolean
  incompleteUpload?: boolean
  uploadError?: boolean
  networkError?: boolean
  failure?: { prefix: string; status: number }
  commands?: string[][]
}

async function runRelease(options: Options = {}) {
  const commands = options.commands || []
  const requests: string[] = []
  const errors: Error[] = []
  const metadata = {
    id: 1,
    tag_name: tag,
    target_commitish: options.wrongDraftTarget ? 'b'.repeat(40) : commit,
    draft: options.draft ?? true,
    assets: options.changedAsset ? [{ ...assets[0], digest: 'sha256:changed' }, ...assets.slice(1)] : assets,
  }
  let existing = options.draft === undefined ? null : metadata
  // Newly created drafts may not appear in tag lookups or listings immediately.
  const listedRelease = existing
  const json = (data: unknown, status = 200) => new Response(JSON.stringify(data), { status })
  await runInNewContext(source, {
    AbortSignal,
    process: {
      env: { GITHUB_REPOSITORY: 'Teddy-Zhu/node-clipboard-rs', GITHUB_SHA: commit, GH_TOKEN: 'test', GH_DEBUG: 'api' },
    },
    console: { log() {}, error: (error: Error) => errors.push(error) },
    fetch: async (url: string, init: RequestInit) => {
      assert.equal((init.headers as Record<string, string>).Authorization, 'Bearer test')
      const endpoint = url.split('/node-clipboard-rs/')[1]
      requests.push(endpoint)
      if (options.networkError) throw new Error('fetch failed')
      if (options.failure && endpoint.startsWith(options.failure.prefix)) {
        // Diagnostic text must not override the actual HTTP status.
        return json({ message: 'Earlier request: (HTTP 404)\nrelease not found' }, options.failure.status)
      }
      if (endpoint.startsWith('git/ref/')) {
        if (options.tag === 'annotated') return json({ object: { type: 'tag', sha: 'annotated' } })
        if (options.tag === 'wrong') return json({ object: { type: 'commit', sha: 'b'.repeat(40) } })
        return options.draft === false ? json({ object: { type: 'commit', sha: commit } }) : json({}, 404)
      }
      if (endpoint === 'git/tags/annotated') return json({ object: { type: 'commit', sha: commit } })
      if (endpoint.startsWith('releases/tags/')) return existing?.draft === false ? json(existing) : json({}, 404)
      if (endpoint === 'releases?per_page=100&page=1') {
        if (options.secondPage) return json(Array.from({ length: 100 }, () => ({ tag_name: 'other', draft: true })))
        return json(listedRelease ? [listedRelease] : [])
      }
      if (endpoint === 'releases?per_page=100&page=2') return json(listedRelease ? [listedRelease] : [])
      if (endpoint === 'releases/1') return json(existing)
      throw new Error(`Unexpected API request: ${endpoint}`)
    },
    require: (name: string) => {
      if (name === './release-manifest.cjs') {
        return {
          loadRelease: () => ({
            commit,
            version: tag.slice(1),
            directory: '/release',
            packages: [{ filename: filenames[0] }],
            binaries: [{ filename: filenames[1] }],
          }),
        }
      }
      if (name === 'node:fs') return { readFileSync: (path: string) => Buffer.from(path) }
      if (name !== 'node:child_process') return require(name)
      return {
        execFileSync: (command: string, args: string[]) => {
          assert.equal(command, 'gh')
          commands.push(args)
          if (args[0] === 'api' && args.includes('POST')) {
            assert.ok(args.includes(`target_commitish=${commit}`))
            assert.ok(args.includes('draft=true'))
            assert.ok(args.includes('generate_release_notes=true'))
            assert.ok(args.includes('prerelease=false'))
            existing = { ...metadata, draft: true, assets: [] }
            return JSON.stringify(existing)
          }
          if (args[0] === 'release' && args[1] === 'upload') {
            assert.equal(args[2], tag)
            assert.ok(args.includes('--clobber'))
            for (const name of filenames) assert.ok(args.includes(join('/release', name)))
            if (options.uploadError) throw new Error('Asset upload failed')
            assert.ok(existing)
            existing.assets = options.incompleteUpload ? assets.slice(1) : assets
            return ''
          }
          if (args[0] === 'api' && args.includes('PATCH')) {
            assert.ok(args.includes('draft=false'))
            assert.ok(existing)
            existing.draft = false
            return JSON.stringify(existing)
          }
          throw Object.assign(new Error('Unexpected CLI lookup'), {
            stderr:
              '* Request to https://api.github.com/releases/tags/v0.0.12\n< HTTP/2.0 404 Not Found\n' +
              '* Request to https://api.github.com/graphql\n< HTTP/2.0 200 OK\n' +
              '{"data":{"repository":{"release":null}}}\nrelease not found\n',
            status: 1,
          })
        },
      }
    },
  })
  if (errors.length) throw errors[0]
  return { commands, requests, existing }
}

test('GitHub Release - 无 tag 和 Release 时可发布，GH_DEBUG 不影响查询', async (t) => {
  const result = await runRelease()
  t.is(result.existing?.draft, false)
  t.is(result.commands.length, 3)
  t.true(result.commands[0].includes('POST'))
  t.true(result.requests.includes('releases/1'))
})

test('GitHub Release - 分页查找并继续上传已有草稿', async (t) => {
  const result = await runRelease({ draft: true, secondPage: true })
  t.true(result.requests.includes('releases?per_page=100&page=2'))
  t.is(result.commands.length, 2)
  t.is(result.existing?.draft, false)
})

test('GitHub Release - 已发布且内容相同时重跑不修改远端', async (t) => {
  const result = await runRelease({ draft: false })
  t.deepEqual(result.commands, [])
})

test('GitHub Release - 支持附注 tag', async (t) => {
  const result = await runRelease({ draft: false, tag: 'annotated' })
  t.true(result.requests.includes('git/tags/annotated'))
  t.deepEqual(result.commands, [])
})

for (const [name, options, message] of [
  ['tag 指向其他提交', { tag: 'wrong' }, /points to another commit/],
  ['草稿指向其他提交', { draft: true, wrongDraftTarget: true }, /Draft release targets another commit/],
  ['已发布内容不同', { draft: false, changedAsset: true }, /Published asset contents differ/],
  ['附件上传失败', { draft: true, uploadError: true }, /Asset upload failed/],
  ['附件不完整', { draft: true, incompleteUpload: true }, /GitHub release assets are incomplete/],
  ['网络故障', { networkError: true }, /fetch failed/],
] as const) {
  test(`GitHub Release - ${name}时阻止发布`, async (t) => {
    const commands: string[][] = []
    await t.throwsAsync(runRelease({ ...options, commands }), { message })
    t.false(commands.some((args) => args.includes('PATCH')))
  })
}

for (const status of [401, 403, 404, 500]) {
  test(`GitHub Release - 列表 API 返回 ${status} 时保留错误，不创建 Release`, async (t) => {
    const commands: string[][] = []
    await t.throwsAsync(runRelease({ failure: { prefix: 'releases?', status }, commands }), {
      message: new RegExp(`HTTP ${status}`),
    })
    t.deepEqual(commands, [])
  })
}
