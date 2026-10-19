# @teddyzhu/clipboard

[![CI](https://github.com/Teddy-Zhu/node-clipboard-rs/actions/workflows/CI.yml/badge.svg)](https://github.com/Teddy-Zhu/node-clipboard-rs/actions/workflows/CI.yml)
[![npm version](https://img.shields.io/npm/v/%40teddyzhu%2Fclipboard)](https://www.npmjs.com/package/@teddyzhu/clipboard)
[![npm downloads](https://img.shields.io/npm/dm/%40teddyzhu%2Fclipboard)](https://www.npmjs.com/package/@teddyzhu/clipboard)
[![GitHub Release](https://img.shields.io/github/v/release/Teddy-Zhu/node-clipboard-rs)](https://github.com/Teddy-Zhu/node-clipboard-rs/releases/latest)
[![License](https://img.shields.io/github/license/Teddy-Zhu/node-clipboard-rs)](./LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/Teddy-Zhu/node-clipboard-rs?style=social)](https://github.com/Teddy-Zhu/node-clipboard-rs/stargazers)

English | [简体中文](./README.zh-CN.md)

A cross-platform Node.js clipboard library built with Rust, with TypeScript support.

- Read and write text, HTML, RTF, images, file lists, and custom formats.
- Write multiple formats together and listen for clipboard changes.
- Read text and images asynchronously.

## Installation

```sh
npm install @teddyzhu/clipboard
```

| Platform      | Prebuilt architectures |
| ------------- | ---------------------- |
| Windows       | x64, arm64             |
| macOS         | x64, arm64             |
| Linux (glibc) | x64, arm64             |

Clipboard access requires a desktop session with access to the system clipboard.
Linux requires X11 or a Wayland compositor supporting `ext-data-control` or `wlr-data-control`.
Linux musl packages are not provided.

## Quick start

```js
const { ClipboardManager } = require('@teddyzhu/clipboard')

const clipboard = new ClipboardManager()
clipboard.setText('Hello from Node.js')
console.log(clipboard.getText())

// Write plain text and HTML together.
clipboard.setContents({
  text: 'Hello',
  html: '<strong>Hello</strong>',
})
```

Reuse a `ClipboardManager` for repeated operations. Top-level functions such as
`getClipboardText()` and `setClipboardText(text)` provide shortcuts without creating an instance.

### Images and files

```js
const { readFileSync } = require('node:fs')
const { resolve } = require('node:path')

// Continue with the clipboard instance above.
clipboard.setImageRaw(readFileSync('image.png'))
const image = clipboard.getImageData() // { width, height, size, data: Buffer }

clipboard.setFiles([resolve('report.txt')])
console.log(clipboard.getFiles())
```

Image readers return PNG data; Base64 readers return the PNG payload without a `data:` prefix.
File setters accept absolute paths or `file://` URIs, and getters return absolute paths.
Use existing local files.

### Listen for changes

```js
const { ClipboardListener } = require('@teddyzhu/clipboard')

const listener = new ClipboardListener()
listener.watch((data) => {
  console.log(data.availableFormats, data.text, data.files)
})

process.once('SIGINT', () => listener.stop())
```

Keep the listener referenced while watching and call `stop()` when finished. It can be restarted after stopping.
Event fields depend on the formats available in the clipboard.

## Common APIs

| Feature          | API                                                               |
| ---------------- | ----------------------------------------------------------------- |
| Text             | `getText()`, `setText(text)`                                      |
| HTML / RTF       | `getHtml()`, `setHtml(html)`, `getRichText()`, `setRichText(rtf)` |
| Images           | `getImageData()`, `setImageRaw(buffer)`, `getImageBase64()`       |
| Files            | `getFiles()`, `setFiles(files)`                                   |
| Custom formats   | `getBuffer(format)`, `setBuffer(format, buffer)`                  |
| Multiple formats | `setContents(contents)`                                           |
| Clear            | `clear()`                                                         |

These methods belong to `ClipboardManager`. The top-level `getFullClipboardData()` function reads a full snapshot,
which can be passed to `setContents()` to restore its readable content.
Async methods include `getTextAsync()`, `setTextAsync(text)`, `getImageBase64Async()`, and `getImageDataAsync()`.

See [index.d.ts](./index.d.ts) for all exports and TypeScript declarations.

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
