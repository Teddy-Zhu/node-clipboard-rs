const assert = require('node:assert/strict')
const { appendFileSync, readFileSync } = require('node:fs')
const { version } = require('../package.json')

assert.ok(process.env.GITHUB_EVENT_PATH && process.env.GITHUB_OUTPUT, 'Run this script from GitHub Actions')
const event = JSON.parse(readFileSync(process.env.GITHUB_EVENT_PATH, 'utf8'))
let publishNpm = false
let publishGithub = false

if (process.env.GITHUB_REF === 'refs/heads/main') {
  if (process.env.GITHUB_EVENT_NAME === 'push') {
    const subject = (event.head_commit?.message ?? '').split(/\r?\n/, 1)[0].trim()
    const release = /^v(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?)$/.exec(subject)
    if (release) {
      assert.equal(release[1], version, 'Release commit version must match package.json')
      publishNpm = true
      publishGithub = true
    }
  } else if (process.env.GITHUB_EVENT_NAME === 'workflow_dispatch') {
    publishNpm = event.inputs?.publish === true || event.inputs?.publish === 'true'
    publishGithub = event.inputs?.github_release === true || event.inputs?.github_release === 'true'
  }
}

appendFileSync(process.env.GITHUB_OUTPUT, `publish_npm=${publishNpm}\npublish_github=${publishGithub}\n`)
console.log(`Release targets: npm=${publishNpm}, GitHub=${publishGithub}`)
