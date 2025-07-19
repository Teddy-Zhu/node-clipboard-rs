# `@teddyzhu/clipboard`

![https://github.com/Teddy-Zhu/node-clipboard-rs/actions](https://github.com/Teddy-Zhu/node-clipboard-rs/workflows/CI/badge.svg)

> it's a node package with napi-rs wrapper clipboard-rs

# Usage

```bash
npm install @teddyzhu/clipboard
```

```javascript
const { ClipboardManager } = require('@teddyzhu/clipboard')

const clipboard = new ClipboardManager()

clipboard.setText('Hello World!')
console.log(clipboard.getText())
```

listen

```javascript
const { ClipboardListener } = require('@teddyzhu/clipboard')

const listener = new ClipboardListener()

listener.watch((data) => {
  console.log('剪贴板数据变化:', data)
  console.log('可用格式:', data.availableFormats)

  if (data.text) {
    console.log('文本:', data.text)
  }

  if (data.image) {
    console.log('图片信息:')
    console.log('  尺寸:', data.image.width + 'x' + data.image.height + 'px')
    console.log('  大小:', data.image.size + ' bytes')
    console.log('  Base64 数据长度:', data.image.base64Data.length + ' 字符')
  }

  if (data.files) {
    console.log('文件:', data.files)
  }
})

// stop listen
listener.stop()
```

## 图片功能增强

现在图片字段包含详细信息：

```javascript
const { ClipboardManager, getClipboardImageData } = require('@teddyzhu/clipboard')

const clipboard = new ClipboardManager()

// 获取图片详细信息
if (clipboard.hasFormat('image')) {
  const imageData = clipboard.getImageData()
  console.log('图片宽度:', imageData.width + 'px')
  console.log('图片高度:', imageData.height + 'px')
  console.log('图片大小:', imageData.size + ' bytes')
  console.log('Base64 数据:', imageData.base64Data)
}

// 异步获取图片数据
const imageDataAsync = await clipboard.getImageDataAsync()

// 快速获取图片数据
const quickImageData = getClipboardImageData()
```
