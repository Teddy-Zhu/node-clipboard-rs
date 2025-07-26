# `@teddyzhu/clipboard`

![https://github.com/Teddy-Zhu/node-clipboard-rs/actions](https://github.com/Teddy-Zhu/node-clipboard-rs/workflows/CI/badge.svg)

> 基于 napi-rs 包装 clipboard-rs 的 Node.js 剪贴板库，提供跨平台剪贴板操作功能

## 特性

- 🚀 跨平台支持（Windows、macOS、Linux）
- 📝 支持多种数据格式：文本、HTML、RTF、图片、文件列表
- 🖼️ 完整的图片处理支持（PNG、JPEG、GIF、BMP）
- 👂 剪贴板实时监听功能
- 🐧 Linux Wayland 环境原生支持
- ⚡ 同步和异步 API
- 🎯 TypeScript 类型定义

## 安装

```bash
npm install @teddyzhu/clipboard
```

## 基本使用

### 剪贴板管理器

```javascript
const { ClipboardManager } = require('@teddyzhu/clipboard')

const clipboard = new ClipboardManager()

// 文本操作
clipboard.setText('Hello World!')
console.log(clipboard.getText()) // "Hello World!"

// HTML 操作
clipboard.setHtml('<h1>Hello HTML</h1>')
console.log(clipboard.getHtml())

// 富文本（RTF）操作
clipboard.setRichText('Hello RTF')
console.log(clipboard.getRichText())

// 检查格式支持
console.log(clipboard.hasFormat('text')) // true
console.log(clipboard.getAvailableFormats()) // ['text', 'html', ...]

// 清空剪贴板
clipboard.clear()
```

### 快速操作函数

```javascript
const {
  getClipboardText,
  setClipboardText,
  getClipboardHtml,
  setClipboardHtml,
  clearClipboard,
  getFullClipboardData,
} = require('@teddyzhu/clipboard')

// 快速文本操作
setClipboardText('Hello World!')
console.log(getClipboardText())

// 快速 HTML 操作
setClipboardHtml('<p>Hello HTML</p>')
console.log(getClipboardHtml())

// 获取完整剪贴板数据
const data = getFullClipboardData()
console.log('可用格式:', data.availableFormats)
console.log('文本内容:', data.text)
console.log('HTML内容:', data.html)

// 清空剪贴板
clearClipboard()
```

## 图片操作

### 基本图片操作

```javascript
const { ClipboardManager, getClipboardImageData } = require('@teddyzhu/clipboard')
const fs = require('fs')

const clipboard = new ClipboardManager()

// 检查是否有图片
if (clipboard.hasFormat('image')) {
  // 获取图片详细信息
  const imageData = clipboard.getImageData()
  console.log('图片宽度:', imageData.width + 'px')
  console.log('图片高度:', imageData.height + 'px')
  console.log('图片大小:', imageData.size + ' bytes')

  // 保存图片到文件
  fs.writeFileSync('clipboard_image.png', imageData.data)

  // 获取 base64 编码
  const base64 = clipboard.getImageBase64()
  console.log('Base64 长度:', base64.length)
}

// 从文件设置图片
const imageBuffer = fs.readFileSync('image.png')
clipboard.setImageRaw(imageBuffer)

// 从 base64 设置图片
const base64Data = fs.readFileSync('image.png', 'base64')
clipboard.setImageBase64(base64Data)

// 快速图片操作
const quickImageData = getClipboardImageData()
```

### 异步图片操作

```javascript
const clipboard = new ClipboardManager()

// 异步获取图片
try {
  const imageData = await clipboard.getImageDataAsync()
  console.log('异步获取图片:', imageData.width + 'x' + imageData.height)

  const base64 = await clipboard.getImageBase64Async()
  console.log('异步获取 Base64 长度:', base64.length)
} catch (error) {
  console.error('获取图片失败:', error.message)
}

// 异步文本操作
await clipboard.setTextAsync('Hello Async!')
const text = await clipboard.getTextAsync()
console.log('异步文本:', text)
```

## 文件操作

```javascript
const { ClipboardManager, getClipboardFiles, setClipboardFiles } = require('@teddyzhu/clipboard')

const clipboard = new ClipboardManager()

// 设置文件列表
const files = ['/path/to/file1.txt', '/path/to/file2.pdf']
clipboard.setFiles(files)

// 获取文件列表
if (clipboard.hasFormat('files')) {
  const clipboardFiles = clipboard.getFiles()
  console.log('剪贴板中的文件:', clipboardFiles)
}

// 快速文件操作
setClipboardFiles(['/path/to/document.pdf'])
const quickFiles = getClipboardFiles()
console.log('快速获取文件:', quickFiles)
```

## 自定义格式数据

```javascript
const clipboard = new ClipboardManager()

// 设置自定义格式数据
const customData = Buffer.from('custom binary data')
clipboard.setBuffer('application/custom', customData)

// 获取自定义格式数据
try {
  const data = clipboard.getBuffer('application/custom')
  console.log('自定义数据:', data.toString())
} catch (error) {
  console.error('自定义格式不存在')
}
```

## 复合内容操作

```javascript
const { ClipboardManager, setClipboardContents } = require('@teddyzhu/clipboard')

const clipboard = new ClipboardManager()

// 同时设置多种格式
const contents = {
  text: 'Hello World!',
  html: '<h1>Hello HTML</h1>',
  rtf: 'Hello RTF',
  // 可选：图片数据
  // image: { width: 100, height: 100, size: 1000, data: imageBuffer },
  // 可选：文件列表
  // files: ['/path/to/file.txt']
}

clipboard.setContents(contents)

// 快速设置复合内容
setClipboardContents({
  text: 'Multi-format content',
  html: '<p>Multi-format <strong>content</strong></p>',
})
```

## 剪贴板监听

### 基本监听

```javascript
const { ClipboardListener } = require('@teddyzhu/clipboard')

const listener = new ClipboardListener()

listener.watch((data) => {
  console.log('剪贴板数据变化:', data)
  console.log('可用格式:', data.availableFormats)

  if (data.text) {
    console.log('文本:', data.text)
  }

  if (data.html) {
    console.log('HTML:', data.html)
  }

  if (data.rtf) {
    console.log('RTF:', data.rtf)
  }

  if (data.image) {
    console.log('图片信息:')
    console.log('  尺寸:', data.image.width + 'x' + data.image.height + 'px')
    console.log('  大小:', data.image.size + ' bytes')
    // 注意：图片数据在 data.image.data (Buffer) 中，不是 base64Data
    console.log('  数据类型:', Buffer.isBuffer(data.image.data) ? 'Buffer' : typeof data.image.data)
  }

  if (data.files) {
    console.log('文件:', data.files)
  }
})

// 检查监听状态
console.log('是否正在监听:', listener.isWatching())
console.log('监听器类型:', listener.getListenerType()) // 'wayland' 或 'generic'

// 停止监听
setTimeout(() => {
  listener.stop()
  console.log('已停止监听')
}, 10000)
```

## Wayland 支持

本库对 Linux Wayland 环境提供原生支持：

```javascript
const { isWaylandClipboardAvailable, ClipboardListener } = require('@teddyzhu/clipboard')

// 检查 Wayland 剪贴板是否可用
if (isWaylandClipboardAvailable()) {
  console.log('Wayland 剪贴板监听可用')

  const listener = new ClipboardListener()
  console.log('当前监听器类型:', listener.getListenerType()) // 在 Wayland 下会显示 'wayland'

  // Wayland 监听器会自动处理不同的 MIME 类型
  listener.watch((data) => {
    console.log('Wayland 剪贴板变化:', data)
  })
} else {
  console.log('使用通用剪贴板监听器')
}
```

## API 参考

### ClipboardManager 类

| 方法                         | 描述                                   |
| ---------------------------- | -------------------------------------- |
| `getText()`                  | 获取纯文本内容                         |
| `setText(text)`              | 设置纯文本内容                         |
| `getHtml()`                  | 获取 HTML 内容                         |
| `setHtml(html)`              | 设置 HTML 内容                         |
| `getRichText()`              | 获取 RTF 富文本内容                    |
| `setRichText(text)`          | 设置 RTF 富文本内容                    |
| `getImageBase64()`           | 获取图片的 base64 编码                 |
| `getImageData()`             | 获取图片详细信息（包含尺寸和原始数据） |
| `setImageBase64(base64Data)` | 从 base64 设置图片                     |
| `setImageRaw(buffer)`        | 从 Buffer 设置图片                     |
| `getImageRaw()`              | 获取图片原始数据（Buffer）             |
| `getFiles()`                 | 获取文件列表                           |
| `setFiles(files)`            | 设置文件列表                           |
| `setBuffer(format, buffer)`  | 设置自定义格式数据                     |
| `getBuffer(format)`          | 获取自定义格式数据                     |
| `setContents(contents)`      | 设置复合内容                           |
| `hasFormat(format)`          | 检查是否包含指定格式                   |
| `getAvailableFormats()`      | 获取所有可用格式                       |
| `clear()`                    | 清空剪贴板                             |

### 异步方法

| 方法                    | 描述                 |
| ----------------------- | -------------------- |
| `getTextAsync()`        | 异步获取文本内容     |
| `setTextAsync(text)`    | 异步设置文本内容     |
| `getImageBase64Async()` | 异步获取图片 base64  |
| `getImageDataAsync()`   | 异步获取图片详细信息 |

### ClipboardListener 类

| 方法                | 描述                                     |
| ------------------- | ---------------------------------------- |
| `watch(callback)`   | 开始监听剪贴板变化                       |
| `stop()`            | 停止监听                                 |
| `isWatching()`      | 检查是否正在监听                         |
| `getListenerType()` | 获取监听器类型（'wayland' 或 'generic'） |

### 快速操作函数

| 函数                                 | 描述                        |
| ------------------------------------ | --------------------------- |
| `getClipboardText()`                 | 快速获取文本                |
| `setClipboardText(text)`             | 快速设置文本                |
| `getClipboardHtml()`                 | 快速获取 HTML               |
| `setClipboardHtml(html)`             | 快速设置 HTML               |
| `getClipboardImage()`                | 快速获取图片（base64）      |
| `getClipboardImageData()`            | 快速获取图片详细信息        |
| `setClipboardImage(base64Data)`      | 快速设置图片（base64）      |
| `setClipboardImageRaw(buffer)`       | 快速设置图片（Buffer）      |
| `getClipboardImageRaw()`             | 快速获取图片原始数据        |
| `getClipboardFiles()`                | 快速获取文件列表            |
| `setClipboardFiles(files)`           | 快速设置文件列表            |
| `getClipboardBuffer(format)`         | 快速获取自定义格式数据      |
| `setClipboardBuffer(format, buffer)` | 快速设置自定义格式数据      |
| `setClipboardContents(contents)`     | 快速设置复合内容            |
| `getFullClipboardData()`             | 快速获取完整剪贴板数据      |
| `clearClipboard()`                   | 快速清空剪贴板              |
| `isWaylandClipboardAvailable()`      | 检查 Wayland 剪贴板是否可用 |

## 数据结构

### ClipboardData

```typescript
interface ClipboardData {
  availableFormats: string[] // 可用的格式列表
  text?: string // 纯文本内容
  rtf?: string // RTF 富文本内容
  html?: string // HTML 内容
  image?: ImageData // 图片数据
  files?: string[] // 文件列表
}
```

### ImageData

```typescript
interface ImageData {
  width: number // 图片宽度（像素）
  height: number // 图片高度（像素）
  size: number // 图片数据大小（字节）
  data: Buffer // 图片原始数据（Buffer）
}
```

## 注意事项

1. **图片格式**：所有图片都会转换为 PNG 格式存储
2. **文件路径**：文件路径需要是绝对路径
3. **自定义格式**：自定义格式的 MIME 类型需要遵循标准
4. **Wayland 支持**：在 Wayland 环境下会自动使用专用监听器以获得更好的性能
5. **异步操作**：对于可能耗时的操作，推荐使用异步版本
6. **错误处理**：所有方法都可能抛出异常，请适当处理错误

## 许可证

MIT
