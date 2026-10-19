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
  run(
    'pwsh',
    [
      '-NoProfile',
      '-NonInteractive',
      '-STA',
      '-Command',
      "$ErrorActionPreference = 'Stop'; Set-Clipboard -Value $env:CLIPBOARD_PROBE; if ((Get-Clipboard -Raw) -cne $env:CLIPBOARD_PROBE) { throw 'Windows clipboard is not accessible in this session' }",
    ],
    { env: { ...process.env, CLIPBOARD_PROBE: probe } },
  )
} else {
  throw new Error(`No clipboard CI environment configured for ${process.platform}`)
}

console.log(`System clipboard environment is ready (${process.platform}/${process.arch})`)
