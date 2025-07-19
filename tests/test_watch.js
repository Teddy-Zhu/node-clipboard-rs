// 测试剪贴板监听功能
const { ClipboardListener } = require('../index')

console.log('开始监听剪贴板变化...')
console.log('请复制一些文本到剪贴板来测试功能')

try {
  const listener = new ClipboardListener()

  listener.watch((data) => {
    console.log('剪贴板数据变化:')
    console.log('  可用格式:', data.availableFormats)

    if (data.text) {
      console.log('  文本:', data.text)
    }

    if (data.html) {
      console.log('  HTML:', data.html.substring(0, 100) + (data.html.length > 100 ? '...' : ''))
    }

    if (data.rtf) {
      console.log('  RTF:', data.rtf.substring(0, 100) + (data.rtf.length > 100 ? '...' : ''))
    }

    if (data.image) {
      console.log('  图片信息:')
      console.log('    - 尺寸:', data.image.width + 'x' + data.image.height + 'px')
      console.log('    - 大小:', data.image.size + ' bytes')
    }

    if (data.files && data.files.length > 0) {
      console.log('  文件:', data.files)
    }

    console.log('---')
  })

  console.log('监听器已启动，按 Ctrl+C 退出')
  console.log('监听状态:', listener.isWatching())

  // 10秒后自动停止监听
  setTimeout(() => {
    console.log('10秒后自动停止监听...')
    listener.stop()
    console.log('监听状态:', listener.isWatching())
    console.log('监听已停止')
    process.exit(0)
  }, 10000)

  // 保持进程运行
  process.stdin.resume()
} catch (error) {
  console.error('启动监听器失败:', error)
}
