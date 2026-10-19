# @teddyzhu/clipboard

[![CI](https://github.com/Teddy-Zhu/node-clipboard-rs/actions/workflows/CI.yml/badge.svg)](https://github.com/Teddy-Zhu/node-clipboard-rs/actions/workflows/CI.yml)
[![npm 版本](https://img.shields.io/npm/v/%40teddyzhu%2Fclipboard)](https://www.npmjs.com/package/@teddyzhu/clipboard)
[![npm 下载量](https://img.shields.io/npm/dm/%40teddyzhu%2Fclipboard)](https://www.npmjs.com/package/@teddyzhu/clipboard)
[![GitHub Release](https://img.shields.io/github/v/release/Teddy-Zhu/node-clipboard-rs)](https://github.com/Teddy-Zhu/node-clipboard-rs/releases/latest)
[![许可证](https://img.shields.io/github/license/Teddy-Zhu/node-clipboard-rs)](./LICENSE)
[![GitHub Stars](https://img.shields.io/github/stars/Teddy-Zhu/node-clipboard-rs?style=social)](https://github.com/Teddy-Zhu/node-clipboard-rs/stargazers)

[English](./README.md) | 简体中文

基于 Rust 和 [napi-rs](https://napi.rs/) 的跨平台 Node.js 剪贴板库。
使用 [clipboard-rs](https://github.com/ChurchTao/clipboard-rs) 调用系统剪贴板 API，
使用 [wl-clipboard-rs](https://github.com/YaLTeR/wl-clipboard-rs) 支持 Linux Wayland。

- 读写文本、HTML、RTF、图片、文件列表和自定义格式。
- 一次写入多种格式，并监听剪贴板变化。
- 提供 TypeScript 类型声明，以及文本和图片的异步读取接口。

## 安装与平台要求

```sh
npm install @teddyzhu/clipboard
```

| 平台            | 预编译架构 | 剪贴板环境要求                        |
| --------------- | ---------- | ------------------------------------- |
| Windows（MSVC） | x64、arm64 | 当前会话具有剪贴板访问权限            |
| macOS           | x64、arm64 | 可以访问系统粘贴板                    |
| Linux（glibc）  | x64、arm64 | X11 显示服务或受支持的 Wayland 合成器 |

CI 在 Node.js 24 下对全部六个平台与架构组合执行完整剪贴板测试，
并在 Linux x64 上复用同一预编译绑定，额外执行 Node.js 22 兼容性测试。
测试把打包后的主包与对应平台包安装到隔离目录，严格检查版本一致性，再验证安装后的公共接口。
`engines.node` 保留 Node-API 4 的兼容范围；其他 Node.js 与平台组合未经 CI 直接验证。
目前不提供 Linux musl 预编译包。

Linux 下，非空的 `WAYLAND_DISPLAY` 或 `XDG_SESSION_TYPE=wayland` 会选择 Wayland 后端。
合成器必须提供 `ext-data-control` 或 `wlr-data-control` 协议；仅存在 Wayland 会话并不代表支持。
可通过 `isWaylandClipboardAvailable()` 检查协议是否可用。其他 Linux 会话使用 X11，需要有效的 `DISPLAY`。
单项读取失败会抛出异常，异步调用会拒绝 Promise；完整快照按格式尽力读取，部分字段可能缺失。

## 快速开始

```js
const { ClipboardManager, getFullClipboardData } = require('@teddyzhu/clipboard')

const clipboard = new ClipboardManager()
clipboard.setText('你好，Node.js')
console.log(clipboard.getText())

// 一次写入多种表示形式，无需传入 availableFormats。
clipboard.setContents({
  text: '你好',
  html: '<strong>你好</strong>',
})
console.log(getFullClipboardData())
```

频繁操作时可复用 `ClipboardManager`。`getClipboardText()`、`setClipboardText(text)` 等顶层函数
提供相应的快捷操作，无需自行创建实例。

### 图片与文件

```js
const { readFileSync } = require('node:fs')
const { resolve } = require('node:path')
const { pathToFileURL } = require('node:url')

// 接续上面的 clipboard 实例。跨平台使用时建议输入 PNG 或 JPEG。
clipboard.setImageRaw(readFileSync('image.png'))
const image = clipboard.getImageData() // { width, height, size, data: Buffer }
// 图片读取接口统一返回 PNG；Base64 接口返回不含 data: 前缀的 PNG 内容。

// 选择本机已存在的文件，macOS 尤其需要文件存在。
const file = resolve('report.txt')
clipboard.setFiles([file])
clipboard.setFiles([pathToFileURL(file).href]) // 也可以传入 file:// URI
console.log(clipboard.getFiles()) // 各平台统一返回本机绝对路径
```

`setFiles()`、`setClipboardFiles()` 和复合写入的 `files` 字段接受本机绝对路径或 `file://` URI。
文件读取、完整快照及监听事件中的文件列表统一返回本机绝对路径。
空格、中文、`%`、`#` 等字符在 Linux URI 列表中正确编码，读取时解码。
相对路径、其他 URI 协议，以及带查询参数或片段的文件 URI 会被拒绝。
Windows 下支持 UNC 路径；Unix 下无法将远程主机的文件 URI 转换为本地路径。

### 监听剪贴板变化

```js
const { ClipboardListener } = require('@teddyzhu/clipboard')

const listener = new ClipboardListener()
listener.watch((data) => {
  console.log(data.availableFormats, data.text, data.files)
})

process.once('SIGINT', () => listener.stop())
```

监听期间保留实例引用，结束时调用 `stop()`；停止后可以重新启动。
初始化失败时 `watch()` 会抛出异常，`isWatching()` 返回当前监听状态。
事件包含变化时可用的格式，具体字段取决于剪贴板所有者和可读取的内容。
启动 Node.js 前设置 `CLIPBOARD_DEBUG=1` 可开启后端诊断日志。

## API 参考

完整的生成类型声明见 [index.d.ts](./index.d.ts)。

| 操作                  | `ClipboardManager` 方法                          | 顶层函数                                                           |
| --------------------- | ------------------------------------------------ | ------------------------------------------------------------------ |
| 文本                  | `getText()`、`setText(text)`                     | `getClipboardText()`、`setClipboardText(text)`                     |
| HTML                  | `getHtml()`、`setHtml(html)`                     | `getClipboardHtml()`、`setClipboardHtml(html)`                     |
| RTF                   | `getRichText()`、`setRichText(rtf)`              | —                                                                  |
| PNG Buffer            | `getImageRaw()`、`setImageRaw(buffer)`           | `getClipboardImageRaw()`、`setClipboardImageRaw(buffer)`           |
| 图片信息与 PNG Buffer | `getImageData()`                                 | `getClipboardImageData()`                                          |
| PNG Base64            | `getImageBase64()`、`setImageBase64(base64)`     | `getClipboardImage()`、`setClipboardImage(base64)`                 |
| 文件列表              | `getFiles()`、`setFiles(files)`                  | `getClipboardFiles()`、`setClipboardFiles(files)`                  |
| 自定义数据            | `getBuffer(format)`、`setBuffer(format, buffer)` | `getClipboardBuffer(format)`、`setClipboardBuffer(format, buffer)` |
| 复合写入              | `setContents(contents)`                          | `setClipboardContents(contents)`                                   |
| 完整快照              | —                                                | `getFullClipboardData()`                                           |
| 格式检查              | `hasFormat(format)`、`getAvailableFormats()`     | —                                                                  |
| 清空                  | `clear()`                                        | `clearClipboard()`                                                 |

实例还提供 `getTextAsync()`、`setTextAsync(text)`、`getImageBase64Async()`、`getImageDataAsync()`，
返回 Promise，并在线程中执行阻塞的剪贴板操作。
`hasFormat()` 接受 `text`、`html`、`rtf`（或 `rich_text`）、`image`、`files`。
`getAvailableFormats()` 返回后端格式名称，可在平台支持时配合自定义 Buffer 接口使用。

`ClipboardListener` 提供 `watch(callback)`、`stop()`、`isWatching()`、`getListenerType()`（`generic` 或 `wayland`）。
`isWaylandClipboardAvailable()` 在非 Linux 平台返回 `false`。

```ts
interface ImageData {
  width: number
  height: number
  size: number
  data: Buffer // 读取时为编码后的 PNG 数据
}

interface ClipboardContents {
  text?: string
  html?: string
  rtf?: string
  image?: ImageData
  files?: string[]
  availableFormats?: string[] // 兼容旧参数，写入时忽略
}

interface ClipboardData extends ClipboardContents {
  availableFormats: string[] // 由读取接口或监听事件提供的格式信息
}
```

复合写入会替换当前剪贴板内容，实际写入格式由提供的内容字段决定，`availableFormats` 不参与格式选择。
可直接把读取到的快照传给 `setContents()`，恢复其中可读取的内容。
传入空的复合对象会清空剪贴板。各平台支持的图片解码格式和自定义格式名称可能不同。

## 开发、测试与发布

源码构建需要 Rust 1.88+、Node.js 22.13+ 或 24+，pnpm 版本以 `package.json` 中的 `packageManager` 为准。

```sh
pnpm install --frozen-lockfile
pnpm build:debug -- --locked
pnpm lint
cargo fmt -- --check
cargo clippy --locked -- -D warnings
```

测试会修改系统剪贴板，并串行执行。Linux 下使用隔离的 X11 显示服务：

```sh
# Debian/Ubuntu 先安装 xvfb、xauth、x11-utils。
XDG_SESSION_TYPE=x11 WAYLAND_DISPLAY= WAYLAND_SOCKET= xvfb-run --auto-servernum \
  --server-args='-screen 0 1280x720x24 -nolisten tcp' \
  sh -c 'node scripts/check-clipboard-env.cjs && pnpm test'
```

macOS/Windows 下在具有剪贴板访问权限的会话中运行 `pnpm test`。
CI 在执行 AVA 前，通过 `pbcopy`/`pbpaste` 检查 macOS 系统粘贴板，通过 PowerShell 检查 Windows 剪贴板。
剪贴板不可用时预检会明确失败，不会静默跳过用例。
Linux CI 覆盖 X11；真实 Wayland 互操作仍需在支持协议的合成器中人工验证。
`tests/` 中的脚本用于补充人工测试。

Push 和 Pull Request 会构建、验证全部六个目标，不执行发布，提交名为 `vX.Y.Z` 也不会发布。
手动运行 workflow 默认同样只构建和验证。
发布步骤：

1. 更新 `package.json` 的主包版本并提交到 `main`。打包任务统一同步平台包及可选依赖版本，
   无需提前把源码锁文件指向尚未发布的平台版本。未发布到 crates.io 的 Rust 包版本与 npm 版本独立。
2. 打开 [Actions → CI → Run workflow](https://github.com/Teddy-Zhu/node-clipboard-rs/actions/workflows/CI.yml)，选择 `main`。
3. 按需勾选 **Publish GitHub Release**、**Publish release to npm**，或同时勾选；两者默认关闭。

六个平台构建、代码检查和安装后的剪贴板测试必须全部通过，才会进入发布任务。
GitHub 发布会以 `package.json` 版本创建 `v<版本号>` 标签，固定到本次 workflow 的提交，并自动生成发布说明。
附件包含六个原生 `.node` 二进制、七个 npm `.tgz` 安装包、`manifest.json` 与 `SHA256SUMS`。
全部附件上传成功后才正式发布；上传失败会保留草稿供重试。已有标签若指向其他提交则明确失败，
已正式发布的 GitHub Release 不会被覆盖。
仅发布 GitHub 使用工作流内置的 `GITHUB_TOKEN`，无需 `NPM_TOKEN`；仓库 Actions 须允许写入 Release。
若启用了不可变 Release，需允许该工作流创建对应版本标签。

npm 发布需要配置对全部七个包具有发布权限的 `NPM_TOKEN` secret，CI 同时启用 npm provenance。
发布使用通过测试的原始安装包，先发布平台包并核对 registry 的完整性摘要，再发布主包。
已存在且摘要一致的版本可以继续执行；同版本内容不同会明确失败。
预发布版本使用 npm 的 `next` 标签及 GitHub 的 prerelease 标识；同时勾选时先发布 npm，再发布 GitHub。
仅通过 GitHub 分发时，需把主包与对应平台的 tarball 一起安装，避免依赖尚未发布的 npm 平台包。

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
