import test from 'ava'
import { ClipboardManager, getClipboardText, setClipboardText, clearClipboard } from '../index'

// 测试数据
const TEST_TEXT = 'Hello, World!'

// ClipboardManager 基本测试
test('ClipboardManager - 创建实例', (t) => {
  const manager = new ClipboardManager()
  t.truthy(manager)
})

test('ClipboardManager - 文本操作', (t) => {
  const manager = new ClipboardManager()

  manager.setText(TEST_TEXT)
  const retrievedText = manager.getText()
  t.is(retrievedText, TEST_TEXT)
})

test('ClipboardManager - 清空剪贴板', (t) => {
  const manager = new ClipboardManager()

  manager.setText(TEST_TEXT)
  manager.clear()
  t.pass() // 清空操作不抛出错误即可
})

// 静态函数测试
test('静态函数 - 文本操作', (t) => {
  setClipboardText(TEST_TEXT)
  const retrievedText = getClipboardText()
  t.is(retrievedText, TEST_TEXT)
})

test('静态函数 - 清空剪贴板', (t) => {
  setClipboardText(TEST_TEXT)
  clearClipboard()
  t.pass() // 清空操作不抛出错误即可
})
