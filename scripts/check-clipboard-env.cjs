const assert = require('node:assert/strict')
const { spawnSync } = require('node:child_process')
const { randomUUID } = require('node:crypto')

function run(command, args, options = {}) {
  const result = spawnSync(command, args, { encoding: 'utf8', timeout: 10000, ...options })
  if (result.error || result.status !== 0) {
    throw new Error(`System clipboard preflight failed (${command}): ${result.error || result.stderr}`)
  }
  return result.stdout
}

const probe = `node-clipboard-rs-${randomUUID()}`
if (process.platform === 'linux') {
  assert.equal(process.env.XDG_SESSION_TYPE, 'x11', 'Linux CI tests require an isolated X11 session')
  assert.ok(process.env.DISPLAY, 'Run Linux CI tests under xvfb-run')
  assert.ok(!process.env.WAYLAND_DISPLAY && !process.env.WAYLAND_SOCKET, 'Wayland must be disabled for X11 tests')
  run('xdpyinfo', [])
} else if (process.platform === 'darwin') {
  run('/usr/bin/pbcopy', [], { input: probe })
  assert.equal(run('/usr/bin/pbpaste', []), probe, 'macOS pasteboard is not accessible in this session')
} else if (process.platform === 'win32') {
  const bindingPath = require.resolve(process.env.CLIPBOARD_TEST_PACKAGE || '../index')
  // Probe Win32 through the tested binding without starting PowerShell's CLR.
  run(process.execPath, [
    '-e',
    `
        const assert = require('node:assert/strict')
        const { ClipboardManager } = require(process.argv[1])
        const clipboard = new ClipboardManager()
        clipboard.setText(process.argv[2])
        assert.equal(clipboard.getText(), process.argv[2], 'Windows clipboard is not accessible in this session')
      `,
    bindingPath,
    probe,
  ])
} else {
  throw new Error(`No clipboard CI environment configured for ${process.platform}`)
}

console.log(`System clipboard environment is ready (${process.platform}/${process.arch})`)
