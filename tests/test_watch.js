// 测试剪贴板监听功能
const { ClipboardListener } = require('../index')

console.log('开始监听剪贴板变化...')
console.log('请复制一些文本到剪贴板来测试功能')

try {
  const listener = new ClipboardListener()

  listener.watch((text) => {
    console.log('剪贴板内容变化:', text)
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
