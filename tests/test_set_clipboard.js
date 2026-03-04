const assert = require('node:assert/strict')
const { ClipboardManager, getClipboardText, setClipboardText } = require('../index')

console.log('开始测试设置剪贴板功能...')

let originalText
try {
  originalText = getClipboardText()
} catch {
  originalText = undefined
}

try {
  const staticText = `test-static-${Date.now()}`
  setClipboardText(staticText)
  assert.equal(getClipboardText(), staticText, '静态 API 设置剪贴板失败')
  console.log('静态 API 设置剪贴板: 通过')

  const manager = new ClipboardManager()
  const managerText = `test-manager-${Date.now()}`
  manager.setText(managerText)
  assert.equal(manager.getText(), managerText, 'ClipboardManager 设置剪贴板失败')
  console.log('ClipboardManager 设置剪贴板: 通过')

  console.log('设置剪贴板功能测试通过')
} catch (error) {
  console.error('设置剪贴板功能测试失败:', error)
  process.exitCode = 1
} finally {
  if (typeof originalText === 'string') {
    try {
      setClipboardText(originalText)
    } catch {
      // 忽略恢复失败，避免影响测试结果
    }
  }
}
