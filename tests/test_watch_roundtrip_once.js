// 手动测试：监听到剪贴板变化后，立即原样回写一次。
// 用于人工验证“监听数据 -> 写回”是否能正确还原剪贴板内容。
const { ClipboardListener, ClipboardManager, getFullClipboardData } = require('../index')

const manager = new ClipboardManager()
const listener = new ClipboardListener()

let hasReplayed = false
let ignoreSelfEventsUntil = 0
let hasShownManualHint = false

function cloneClipboardData(data) {
  return {
    availableFormats: Array.isArray(data.availableFormats) ? [...data.availableFormats] : [],
    text: data.text,
    rtf: data.rtf,
    html: data.html,
    image: data.image
      ? {
          width: data.image.width,
          height: data.image.height,
          size: data.image.size,
          data: Buffer.from(data.image.data),
        }
      : undefined,
    files: Array.isArray(data.files) ? [...data.files] : undefined,
  }
}

function formatSummary(data) {
  const formats = Array.isArray(data.availableFormats) ? data.availableFormats.join(', ') : ''
  return {
    availableFormats: formats || '(空)',
    hasText: typeof data.text === 'string',
    textLength: typeof data.text === 'string' ? data.text.length : 0,
    hasHtml: typeof data.html === 'string',
    htmlLength: typeof data.html === 'string' ? data.html.length : 0,
    hasRtf: typeof data.rtf === 'string',
    rtfLength: typeof data.rtf === 'string' ? data.rtf.length : 0,
    hasFiles: Array.isArray(data.files),
    fileCount: Array.isArray(data.files) ? data.files.length : 0,
    hasImage: Boolean(data.image),
    imageBytes: data.image?.data?.length ?? 0,
  }
}

function areStringArraysEqual(a, b) {
  const left = Array.isArray(a) ? [...a].sort() : []
  const right = Array.isArray(b) ? [...b].sort() : []
  return JSON.stringify(left) === JSON.stringify(right)
}

function compareData(expected, actual) {
  const bothHaveImage = Boolean(expected.image) && Boolean(actual.image)
  const imageEqual = !expected.image && !actual.image
    ? true
    : bothHaveImage &&
      Buffer.isBuffer(expected.image.data) &&
      Buffer.isBuffer(actual.image.data) &&
      expected.image.data.equals(actual.image.data)

  return {
    availableFormats: areStringArraysEqual(expected.availableFormats, actual.availableFormats),
    text: (expected.text ?? null) === (actual.text ?? null),
    html: (expected.html ?? null) === (actual.html ?? null),
    rtf: (expected.rtf ?? null) === (actual.rtf ?? null),
    files: areStringArraysEqual(expected.files, actual.files),
    image: imageEqual,
  }
}

function printManualHint() {
  if (hasShownManualHint) return
  hasShownManualHint = true
  console.log('\n请现在到目标应用中粘贴并人工确认内容是否正确还原。')
  console.log('确认后按回车结束脚本（或 Ctrl+C 退出）。\n')
}

function cleanup(exitCode = 0) {
  try {
    listener.stop()
  } catch {
    // 忽略停止失败
  }
  process.exit(exitCode)
}

console.log('启动监听：收到第一次剪贴板变化后，将自动原样回写一次。')
console.log('请先复制一段你要验证的内容（文本/HTML/RTF/图片/文件）。\n')

listener.watch((data) => {
  const now = Date.now()
  if (now < ignoreSelfEventsUntil) {
    console.log('忽略本次事件（回写触发的自事件）')
    return
  }

  if (hasReplayed) {
    console.log('已完成一次回写，本次事件仅打印摘要：', formatSummary(data))
    printManualHint()
    return
  }

  hasReplayed = true
  const captured = cloneClipboardData(data)

  console.log('收到首次剪贴板变化，捕获摘要：', formatSummary(captured))
  console.log('开始执行一次原样回写...')

  try {
    manager.setContents(captured)
    ignoreSelfEventsUntil = Date.now() + 1500
    console.log('回写完成。')
  } catch (error) {
    console.error('回写失败：', error)
    cleanup(1)
    return
  }

  setTimeout(() => {
    try {
      const afterWrite = cloneClipboardData(getFullClipboardData())
      console.log('回写后摘要：', formatSummary(afterWrite))
      console.log('回写前后字段一致性：', compareData(captured, afterWrite))
      printManualHint()
    } catch (error) {
      console.error('读取回写后的剪贴板失败：', error)
      cleanup(1)
    }
  }, 350)
})

process.stdin.setEncoding('utf8')
process.stdin.resume()
process.stdin.once('data', () => cleanup(0))
process.on('SIGINT', () => cleanup(0))
