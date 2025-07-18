const { ClipboardListener, ClipboardManager } = require('../index')

console.log('测试剪贴板监听器的新功能...')

// 创建剪贴板管理器用于设置测试数据
const manager = new ClipboardManager()

// 创建监听器
const listener = new ClipboardListener()

// 开始监听
listener.watch((data) => {
  console.log('\n=== 剪贴板数据变化 ===')
  console.log('可用格式:', data.availableFormats)

  if (data.text) {
    console.log('文本内容:', data.text)
  }

  if (data.html) {
    console.log('HTML 内容:', data.html)
  }

  if (data.rtf) {
    console.log('RTF 内容:', data.rtf)
  }

  if (data.image) {
    console.log('图片数据 (base64, 前50字符):', data.image.substring(0, 50) + '...')
    console.log('图片数据长度:', data.image.length)
  }

  if (data.files && data.files.length > 0) {
    console.log('文件列表:', data.files)
  }

  if (data.other && Object.keys(data.other).length > 0) {
    console.log('其他格式数据:', data.other)
  }

  console.log('========================\n')
})

console.log('监听器已启动！')
console.log('请尝试以下操作来测试：')
console.log('1. 复制一些文本')
console.log('2. 复制包含格式的富文本')
console.log('3. 复制一张图片')
console.log('4. 复制一些文件')
console.log('\n按 Ctrl+C 退出...\n')

// 设置一些测试数据
setTimeout(() => {
  console.log('设置测试文本...')
  manager.setText('这是一个测试文本xxxx')
}, 2000)

setTimeout(() => {
  console.log('设置测试 HTML...')
  manager.setHtml('<p><strong>这是一个</strong> <em>HTML</em> 测试</p>')
}, 4000)

setTimeout(() => {
  console.log('设置测试富文本...')
  manager.setRichText('这是一个富文本测试')
}, 6000)

// 优雅退出
process.on('SIGINT', () => {
  console.log('\n正在停止监听器...')
  listener.stop()
  console.log('监听器已停止。再见！')
  process.exit(0)
})
