const assert = require('node:assert/strict')
const { readFileSync, readdirSync, statSync } = require('node:fs')
const { join } = require('node:path')
const { version, name, napi, optionalDependencies } = require('../package.json')

const packages = readdirSync('npm')
assert.equal(packages.length, napi.targets.length, 'Platform package count must match napi.targets')
assert.equal(Object.keys(optionalDependencies).length, napi.targets.length, 'Unexpected optional dependencies')

for (const platform of packages) {
  const directory = join('npm', platform)
  const pkg = JSON.parse(readFileSync(join(directory, 'package.json'), 'utf8'))
  assert.equal(pkg.name, `${name}-${platform}`)
  assert.equal(pkg.version, version)
  assert.equal(optionalDependencies[pkg.name], version)
  assert.equal(pkg.main, `${napi.binaryName}.${platform}.node`)
  assert.ok(statSync(join(directory, pkg.main)).size > 0, `Missing binary: ${pkg.name}`)
}

console.log(`Release artifacts ready: ${packages.length} platform packages at ${version}`)
