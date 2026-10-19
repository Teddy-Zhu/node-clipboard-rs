import test from 'ava'
import { spawnSync } from 'node:child_process'
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { createRequire } from 'node:module'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { pathToFileURL } from 'node:url'

const require = createRequire(import.meta.url)
const bindingPath = require.resolve(process.env.CLIPBOARD_TEST_PACKAGE || '../index')
const {
  ClipboardManager,
  getClipboardText,
  setClipboardText,
  clearClipboard,
  getClipboardFiles,
  setClipboardFiles,
  getFullClipboardData,
  setClipboardContents,
} = require(bindingPath) as typeof import('../index')

// 测试数据
const TEST_TEXT = 'Hello, World!'

// ClipboardManager 基本测试
test.serial('ClipboardManager - 创建实例', (t) => {
  const manager = new ClipboardManager()
  t.truthy(manager)
})

test.serial('ClipboardManager - 文本操作', (t) => {
  const manager = new ClipboardManager()

  manager.setText(TEST_TEXT)
  const retrievedText = manager.getText()
  t.is(retrievedText, TEST_TEXT)
})

test.serial('ClipboardManager - 清空剪贴板', (t) => {
  const manager = new ClipboardManager()

  manager.setText(TEST_TEXT)
  manager.clear()
  t.pass() // 清空操作不抛出错误即可
})

// 静态函数测试
test.serial('静态函数 - 文本操作', (t) => {
  setClipboardText(TEST_TEXT)
  const retrievedText = getClipboardText()
  t.is(retrievedText, TEST_TEXT)
})

test.serial('静态函数 - 清空剪贴板', (t) => {
  setClipboardText(TEST_TEXT)
  clearClipboard()
  t.pass() // 清空操作不抛出错误即可
})

test.serial('ClipboardManager - 异步文本往返', async (t) => {
  const manager = new ClipboardManager()
  await manager.setTextAsync(TEST_TEXT)
  t.is(await manager.getTextAsync(), TEST_TEXT)
})

test.serial('复合内容 - 无需 availableFormats，读取结果可以直接恢复', (t) => {
  const manager = new ClipboardManager()
  const html = '<strong>Hello clipboard</strong>'
  manager.setContents({ text: TEST_TEXT, html })
  t.is(manager.getText(), TEST_TEXT)
  t.true(manager.getHtml().includes(html))

  const snapshot = getFullClipboardData()
  t.true(snapshot.availableFormats.includes('text'))
  manager.clear()
  setClipboardContents(snapshot)
  t.is(manager.getText(), TEST_TEXT)
  t.true(manager.getHtml().includes(html))

  setClipboardContents({ text: 'Static composite content' })
  t.is(manager.getText(), 'Static composite content')
  manager.setContents({ text: TEST_TEXT, availableFormats: [] })
  t.is(manager.getText(), TEST_TEXT)
})

test.serial('文件列表 - 路径与 URI 写入、快照读取和 Linux URI 编码一致', (t) => {
  const manager = new ClipboardManager()
  const directory = mkdtempSync(join(tmpdir(), 'clipboard-files-'))
  t.teardown(() => rmSync(directory, { recursive: true, force: true }))
  const paths = [join(directory, '报告 100% #1.txt'), join(directory, 'second file.txt')]
  for (const path of paths) writeFileSync(path, 'clipboard file test')
  const uris = paths.map((path) => pathToFileURL(path).href)

  manager.setFiles(paths)
  t.deepEqual(manager.getFiles(), paths)
  t.deepEqual(getClipboardFiles(), paths)
  if (process.platform === 'linux') {
    t.deepEqual(manager.getBuffer('text/uri-list').toString('utf8').split(/\r?\n/).filter(Boolean), uris)
  }

  setClipboardFiles(uris)
  t.deepEqual(manager.getFiles(), paths)
  manager.setContents({ files: uris })
  t.deepEqual(getFullClipboardData().files, paths)
  setClipboardContents({ files: paths })
  t.deepEqual(getClipboardFiles(), paths)

  if (process.platform === 'linux') {
    manager.setBuffer('text/uri-list', Buffer.from(`# File list\r\n${uris.join('\r\n')}\r\n`))
    t.deepEqual(manager.getFiles(), paths)
    t.deepEqual(getFullClipboardData().files, paths)
  }
})

test.serial('文件列表 - 无效输入抛错且不覆盖原内容', (t) => {
  const manager = new ClipboardManager()
  manager.setText(TEST_TEXT)
  const uri = pathToFileURL(join(tmpdir(), 'clipboard.txt')).href
  for (const file of ['relative.txt', 'https://example.com/file.txt', `${uri}?query=1`, `${uri}#fragment`]) {
    const error = { message: /absolute paths|without queries or fragments/ }
    t.throws(() => manager.setFiles([file]), error)
    t.throws(() => setClipboardFiles([file]), error)
    t.throws(() => manager.setContents({ files: [file] }), error)
    t.throws(() => setClipboardContents({ files: [file] }), error)
  }
  t.is(manager.getText(), TEST_TEXT)
})

test.serial('ClipboardListener - 空闲时停止和重启后进程正常退出', (t) => {
  const result = spawnSync(
    process.execPath,
    [
      '-e',
      `
    const assert = require('node:assert/strict')
    const { ClipboardListener } = require(process.argv[1])
    const listener = new ClipboardListener()
    for (let i = 0; i < 2; i++) {
      listener.watch(() => {})
      assert.equal(listener.isWatching(), true)
      listener.stop()
      assert.equal(listener.isWatching(), false)
    }
  `,
      bindingPath,
    ],
    { timeout: 10000, encoding: 'utf8' },
  )
  t.is(result.error, undefined)
  t.is(result.status, 0, result.stderr)
})

if (process.platform === 'linux') {
  test.serial('ClipboardListener - 启动失败抛错且不保留监听状态', (t) => {
    for (const wayland of [false, true]) {
      const result = spawnSync(
        process.execPath,
        [
          '-e',
          `
        const assert = require('node:assert/strict')
        const { ClipboardListener } = require(process.argv[1])
        const listener = new ClipboardListener()
        assert.throws(() => listener.watch(() => {}), /Failed to/)
        assert.equal(listener.isWatching(), false)
        listener.stop()
      `,
          bindingPath,
        ],
        {
          timeout: 10000,
          encoding: 'utf8',
          env: {
            ...process.env,
            DISPLAY: '',
            WAYLAND_DISPLAY: wayland ? '/nonexistent-clipboard-test-wayland' : '',
            XDG_SESSION_TYPE: wayland ? 'wayland' : 'x11',
          },
        },
      )
      t.is(result.error, undefined)
      t.is(result.status, 0, result.stderr)
    }
  })
}
