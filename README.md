# @teddyzhu/clipboard

[![CI](https://github.com/Teddy-Zhu/node-clipboard-rs/actions/workflows/CI.yml/badge.svg)](https://github.com/Teddy-Zhu/node-clipboard-rs/actions/workflows/CI.yml)

English | [简体中文](./README.zh-CN.md)

Cross-platform Node.js clipboard access and monitoring, built with Rust and [napi-rs](https://napi.rs/).
Uses [clipboard-rs](https://github.com/ChurchTao/clipboard-rs) for native desktop APIs and
[wl-clipboard-rs](https://github.com/YaLTeR/wl-clipboard-rs) for Linux Wayland.

- Read and write text, HTML, RTF, images, file lists, and custom formats.
- Write several formats together and listen for clipboard changes.
- Includes TypeScript declarations and async text/image readers.

## Install and platform requirements

```sh
npm install @teddyzhu/clipboard
```

| Platform       | Prebuilt architectures | Clipboard requirement                          |
| -------------- | ---------------------- | ---------------------------------------------- |
| Windows (MSVC) | x64, arm64             | Access to the current session's clipboard      |
| macOS          | x64, arm64             | Access to the system pasteboard                |
| Linux (glibc)  | x64, arm64             | X11 display, or a supported Wayland compositor |

CI runs on Node.js 20, 22, and 24. The `engines.node` range retains Node-API 4 compatibility;
older Node.js versions are outside the CI test matrix. Linux musl binaries are not provided.

On Linux, a nonempty `WAYLAND_DISPLAY` or `XDG_SESSION_TYPE=wayland` selects Wayland.
The compositor must expose `ext-data-control` or `wlr-data-control`; a Wayland session alone does not guarantee support.
Use `isWaylandClipboardAvailable()` to check protocol availability. Other Linux sessions use X11 and require a valid `DISPLAY`.
Clipboard read failures throw; async calls reject. Full snapshots read supported formats on a best-effort basis, so individual fields may be absent.

## Quick start

```js
const { ClipboardManager, getFullClipboardData } = require('@teddyzhu/clipboard')

const clipboard = new ClipboardManager()
clipboard.setText('Hello from Node.js')
console.log(clipboard.getText())

// Write multiple representations in one operation. availableFormats is not required.
clipboard.setContents({
  text: 'Hello',
  html: '<strong>Hello</strong>',
})
console.log(getFullClipboardData())
```

Reuse a `ClipboardManager` for repeated operations. Top-level functions such as `getClipboardText()` and
`setClipboardText(text)` provide the same operations without managing an instance.

### Images and files

```js
const { readFileSync } = require('node:fs')
const { resolve } = require('node:path')
const { pathToFileURL } = require('node:url')

// Continue with the clipboard instance above. PNG and JPEG are portable input formats.
clipboard.setImageRaw(readFileSync('image.png'))
const image = clipboard.getImageData() // { width, height, size, data: Buffer }
// Image readers return PNG data. Base64 readers return the PNG payload without a data: prefix.

// Choose an existing local file, particularly on macOS.
const file = resolve('report.txt')
clipboard.setFiles([file])
clipboard.setFiles([pathToFileURL(file).href]) // file:// URIs are accepted too
console.log(clipboard.getFiles()) // Native absolute paths on every platform
```

`setFiles()`, `setClipboardFiles()`, and the `files` field of composite writes accept native absolute paths or
`file://` URIs. File getters, full snapshots, and listener events return native absolute paths.
Spaces, Unicode, `%`, and `#` are encoded for Linux URI lists and decoded on reads.
Relative paths, other URI schemes, and file URIs with queries/fragments are rejected.
Windows UNC paths are supported on Windows; remote-host file URIs cannot be represented as local paths on Unix.

### Clipboard changes

```js
const { ClipboardListener } = require('@teddyzhu/clipboard')

const listener = new ClipboardListener()
listener.watch((data) => {
  console.log(data.availableFormats, data.text, data.files)
})

process.once('SIGINT', () => listener.stop())
```

Keep the listener referenced while watching and call `stop()` when finished. A stopped listener can be restarted.
`watch()` throws if initialization fails; `isWatching()` reports the current state.
Events contain the formats available at the time of the change; optional fields depend on the clipboard owner and readable formats.
Set `CLIPBOARD_DEBUG=1` before starting Node.js to enable backend diagnostics.

## API reference

The complete generated signatures are in [index.d.ts](./index.d.ts).

| Operation                   | `ClipboardManager` methods                       | Top-level functions                                                |
| --------------------------- | ------------------------------------------------ | ------------------------------------------------------------------ |
| Text                        | `getText()`, `setText(text)`                     | `getClipboardText()`, `setClipboardText(text)`                     |
| HTML                        | `getHtml()`, `setHtml(html)`                     | `getClipboardHtml()`, `setClipboardHtml(html)`                     |
| RTF                         | `getRichText()`, `setRichText(rtf)`              | —                                                                  |
| PNG buffer                  | `getImageRaw()`, `setImageRaw(buffer)`           | `getClipboardImageRaw()`, `setClipboardImageRaw(buffer)`           |
| Image metadata + PNG buffer | `getImageData()`                                 | `getClipboardImageData()`                                          |
| PNG base64                  | `getImageBase64()`, `setImageBase64(base64)`     | `getClipboardImage()`, `setClipboardImage(base64)`                 |
| Files                       | `getFiles()`, `setFiles(files)`                  | `getClipboardFiles()`, `setClipboardFiles(files)`                  |
| Custom data                 | `getBuffer(format)`, `setBuffer(format, buffer)` | `getClipboardBuffer(format)`, `setClipboardBuffer(format, buffer)` |
| Composite write             | `setContents(contents)`                          | `setClipboardContents(contents)`                                   |
| Full snapshot               | —                                                | `getFullClipboardData()`                                           |
| Format inspection           | `hasFormat(format)`, `getAvailableFormats()`     | —                                                                  |
| Clear                       | `clear()`                                        | `clearClipboard()`                                                 |

Async instance methods: `getTextAsync()`, `setTextAsync(text)`, `getImageBase64Async()`, and `getImageDataAsync()`.
They return promises and perform blocking clipboard work away from the JavaScript thread.
`hasFormat()` accepts `text`, `html`, `rtf` (or `rich_text`), `image`, and `files`.
`getAvailableFormats()` returns backend-specific names; use them with custom buffer operations where supported.

`ClipboardListener` exposes `watch(callback)`, `stop()`, `isWatching()`, and `getListenerType()` (`generic` or `wayland`).
`isWaylandClipboardAvailable()` returns `false` outside Linux.

```ts
interface ImageData {
  width: number
  height: number
  size: number
  data: Buffer // Encoded PNG when returned by a reader
}

interface ClipboardContents {
  text?: string
  html?: string
  rtf?: string
  image?: ImageData
  files?: string[]
  availableFormats?: string[] // Accepted for compatibility; ignored on writes
}

interface ClipboardData extends ClipboardContents {
  availableFormats: string[] // Format metadata supplied by readers/listeners
}
```

Composite writes replace the current clipboard contents. Only provided content fields determine the formats written;
`availableFormats` does not select formats. Pass a snapshot directly to `setContents()` to restore its readable content.
An empty composite object clears the clipboard. Platform-specific image decoders and custom format names may differ.

## Development, tests, and release

Source builds require Rust 1.88+ and Node.js 22.13+ or 24+, with pnpm matching `packageManager` in `package.json`.

```sh
pnpm install --frozen-lockfile
pnpm build:debug -- --locked
pnpm lint
cargo fmt -- --check
cargo clippy --locked -- -D warnings
```

Tests change the system clipboard and run serially. On Linux, use an isolated X11 display:

```sh
# Debian/Ubuntu: install xvfb, xauth, and x11-utils first.
XDG_SESSION_TYPE=x11 WAYLAND_DISPLAY= WAYLAND_SOCKET= xvfb-run --auto-servernum \
  --server-args='-screen 0 1280x720x24 -nolisten tcp' \
  sh -c 'node scripts/check-clipboard-env.cjs && pnpm test'
```

On macOS/Windows, run `pnpm test` in a session with clipboard access.
CI checks macOS pasteboard access with `pbcopy`/`pbpaste` and Windows clipboard access with PowerShell before running AVA.
An unavailable clipboard fails the preflight instead of silently skipping tests.
The Linux CI suite covers X11; actual Wayland interoperability still needs a supported compositor and manual validation.
Scripts in `tests/` are supplementary manual scenarios.

Pushes and pull requests build and validate all six targets; they never publish, including commits named `vX.Y.Z`.
Manual workflow runs also default to build and validation only.
To release, prepare a new package version and its lockfiles, then run **Actions → CI → Run workflow** on `main`
and explicitly enable **Publish release to npm**. All builds, checks, and clipboard tests must pass before publishing.
The publish job validates every platform binary, publishes platform packages first, verifies their registry versions,
then publishes the main package. It requires an `NPM_TOKEN` secret with publish access.

## License

[MIT](./LICENSE)
