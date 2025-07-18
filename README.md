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

listener.watch((text) => {
  console.log('剪贴板内容变化:', text)
})

// stop listen
listener.stop()
```
