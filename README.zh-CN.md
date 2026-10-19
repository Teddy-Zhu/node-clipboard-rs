# @teddyzhu/clipboard

[![CI](https://github.com/Teddy-Zhu/node-clipboard-rs/actions/workflows/CI.yml/badge.svg)](https://github.com/Teddy-Zhu/node-clipboard-rs/actions/workflows/CI.yml)
[![npm 版本](https://img.shields.io/npm/v/%40teddyzhu%2Fclipboard)](https://www.npmjs.com/package/@teddyzhu/clipboard)
[![npm 下载量](https://img.shields.io/npm/dm/%40teddyzhu%2Fclipboard)](https://www.npmjs.com/package/@teddyzhu/clipboard)
[![GitHub Release](https://img.shields.io/github/v/release/Teddy-Zhu/node-clipboard-rs)](https://github.com/Teddy-Zhu/node-clipboard-rs/releases/latest)
[![许可证](https://img.shields.io/github/license/Teddy-Zhu/node-clipboard-rs)](./LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/Teddy-Zhu/node-clipboard-rs?style=social)](https://github.com/Teddy-Zhu/node-clipboard-rs/stargazers)

[English](./README.md) | 简体中文

基于 Rust 的跨平台 Node.js 剪贴板库，提供 TypeScript 类型声明。

- 读写文本、HTML、RTF、图片、文件列表和自定义格式。
- 一次写入多种格式，并监听剪贴板变化。
- 支持异步读取文本和图片。

## 安装

```sh
npm install @teddyzhu/clipboard
```

| 平台           | 预编译架构 |
| -------------- | ---------- |
| Windows        | x64、arm64 |
| macOS          | x64、arm64 |
| Linux（glibc） | x64、arm64 |

需要在可访问系统剪贴板的桌面会话中使用。
Linux 需要 X11，或支持 `ext-data-control` / `wlr-data-control` 协议的 Wayland 合成器。
暂不提供 Linux musl 预编译包。

## 快速开始

```js
const { ClipboardManager } = require('@teddyzhu/clipboard')

const clipboard = new ClipboardManager()
clipboard.setText('你好，Node.js')
console.log(clipboard.getText())

// 同时写入纯文本和 HTML。
clipboard.setContents({
  text: '你好',
  html: '<strong>你好</strong>',
})
```

频繁操作时可复用 `ClipboardManager`。也可以使用 `getClipboardText()`、`setClipboardText(text)`
等顶层函数，无需创建实例。

### 图片与文件

```js
const { readFileSync } = require('node:fs')
const { resolve } = require('node:path')

// 接续上面的 clipboard 实例。
clipboard.setImageRaw(readFileSync('image.png'))
const image = clipboard.getImageData() // { width, height, size, data: Buffer }

clipboard.setFiles([resolve('report.txt')])
console.log(clipboard.getFiles())
```

图片读取接口返回 PNG 数据；Base64 接口返回不含 `data:` 前缀的 PNG 内容。
文件写入接受绝对路径或 `file://` URI，读取返回绝对路径。请使用本机已存在的文件。

### 监听变化

```js
const { ClipboardListener } = require('@teddyzhu/clipboard')

const listener = new ClipboardListener()
listener.watch((data) => {
  console.log(data.availableFormats, data.text, data.files)
})

process.once('SIGINT', () => listener.stop())
```

监听期间保留实例引用，结束时调用 `stop()`；停止后可重新启动。
事件中的具体字段取决于剪贴板中可读取的格式。

## 常用 API

| 功能       | 接口                                                              |
| ---------- | ----------------------------------------------------------------- |
| 文本       | `getText()`、`setText(text)`                                      |
| HTML / RTF | `getHtml()`、`setHtml(html)`、`getRichText()`、`setRichText(rtf)` |
| 图片       | `getImageData()`、`setImageRaw(buffer)`、`getImageBase64()`       |
| 文件       | `getFiles()`、`setFiles(files)`                                   |
| 自定义格式 | `getBuffer(format)`、`setBuffer(format, buffer)`                  |
| 多格式写入 | `setContents(contents)`                                           |
| 清空       | `clear()`                                                         |

以上为 `ClipboardManager` 的实例方法。顶层函数 `getFullClipboardData()` 可读取完整快照，
并可将结果传给 `setContents()` 恢复其中可读取的内容。
异步接口包括 `getTextAsync()`、`setTextAsync(text)`、`getImageBase64Async()`、`getImageDataAsync()`。

全部导出与 TypeScript 类型声明见 [index.d.ts](./index.d.ts)。

## 致谢

感谢以下开源项目的维护者与贡献者，为本项目提供核心能力和开发工具：

- [clipboard-rs](https://github.com/ChurchTao/clipboard-rs)：系统剪贴板读写与变更通知。
- [wl-clipboard-rs](https://github.com/YaLTeR/wl-clipboard-rs)：Wayland 剪贴板读写与监听。
- [napi-rs](https://github.com/napi-rs/napi-rs)：Node-API 的 Rust 绑定、TypeScript 类型生成与原生包构建工具。
- [Tokio](https://github.com/tokio-rs/tokio)：异步运行时集成。
- [rust-url](https://github.com/servo/rust-url) 和 [rust-base64](https://github.com/marshallpierce/rust-base64)：文件 URL 处理与 Base64 编码。
- [AVA](https://github.com/avajs/ava)：公共接口与剪贴板集成测试。

## 许可证

[MIT](./LICENSE)
