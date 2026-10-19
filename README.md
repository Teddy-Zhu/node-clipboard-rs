# @teddyzhu/clipboard

[![CI](https://github.com/Teddy-Zhu/node-clipboard-rs/actions/workflows/CI.yml/badge.svg)](https://github.com/Teddy-Zhu/node-clipboard-rs/actions/workflows/CI.yml)
[![npm version](https://img.shields.io/npm/v/%40teddyzhu%2Fclipboard)](https://www.npmjs.com/package/@teddyzhu/clipboard)
[![npm downloads](https://img.shields.io/npm/dm/%40teddyzhu%2Fclipboard)](https://www.npmjs.com/package/@teddyzhu/clipboard)
[![GitHub Release](https://img.shields.io/github/v/release/Teddy-Zhu/node-clipboard-rs)](https://github.com/Teddy-Zhu/node-clipboard-rs/releases/latest)
[![License](https://img.shields.io/github/license/Teddy-Zhu/node-clipboard-rs)](./LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/Teddy-Zhu/node-clipboard-rs?style=social)](https://github.com/Teddy-Zhu/node-clipboard-rs/stargazers)

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

CI runs the full clipboard test suite on Node.js 24 for all six platform/architecture targets,
plus a Node.js 22 compatibility run on Linux x64 using the same prebuilt binding.
These tests install the packed main package and its matching native package into an isolated directory,
enforce package version consistency, and exercise the installed public API.
The `engines.node` range retains Node-API 4 compatibility; other Node.js/platform combinations are not directly tested in CI.
Linux musl binaries are not provided.

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
To release:

1. Update the main version in `package.json` and commit it to `main`. Platform package versions and optional dependency
   versions are synchronized during packaging; do not point the source lockfile at unpublished platform versions.
   The unpublished Rust crate version is independent of the npm package version.
2. Open [Actions → CI → Run workflow](https://github.com/Teddy-Zhu/node-clipboard-rs/actions/workflows/CI.yml) and select `main`.
3. Enable **Publish GitHub Release**, **Publish release to npm**, or both. Both options default to off.

All six builds, checks, and installed-package clipboard tests must pass before publishing.
The GitHub option creates `v<package.json version>` at the workflow's exact commit, generates release notes, and uploads
six native `.node` binaries, seven npm `.tgz` packages, `manifest.json`, and `SHA256SUMS`.
The release stays a draft until every asset has uploaded successfully. An existing tag pointing to another commit blocks
publication; a failed draft upload can be retried. Published GitHub releases are not overwritten.
GitHub-only releases use the workflow's `GITHUB_TOKEN`; no `NPM_TOKEN` is required. Repository Actions must allow write
access to releases. If immutable releases are enabled, allow the workflow to create the version tag.

The npm option requires an `NPM_TOKEN` secret with publish access to all seven packages and npm provenance enabled by CI.
It publishes the exact tested tarballs: platform packages first, then the main package after registry integrity checks.
Matching existing package versions can be resumed; differing contents at an existing version fail explicitly.
Prerelease versions use the npm `next` tag and GitHub's prerelease flag. When both options are enabled, npm publishes first.
For GitHub-only distribution, install the main tarball together with the matching platform tarball so npm can resolve
the native dependency without a corresponding npm release.

## Acknowledgements

Thanks to the maintainers and contributors of the open source projects that make this package possible:

- [clipboard-rs](https://github.com/ChurchTao/clipboard-rs): native desktop clipboard access and change notifications.
- [wl-clipboard-rs](https://github.com/YaLTeR/wl-clipboard-rs): Wayland clipboard access and monitoring.
- [napi-rs](https://github.com/napi-rs/napi-rs): Rust bindings for Node-API, TypeScript declarations, and native package tooling.
- [Tokio](https://github.com/tokio-rs/tokio): async runtime integration.
- [rust-url](https://github.com/servo/rust-url) and [rust-base64](https://github.com/marshallpierce/rust-base64): file URL handling and Base64 encoding.
- [AVA](https://github.com/avajs/ava): public API and clipboard integration tests.

## License

[MIT](./LICENSE)
