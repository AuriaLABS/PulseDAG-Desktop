import fs from 'node:fs'

const expectedVersion = '0.1.0'
const packageJson = JSON.parse(fs.readFileSync('package.json', 'utf8'))
const tauriConfig = JSON.parse(fs.readFileSync('src-tauri/tauri.conf.json', 'utf8'))
const cargoToml = fs.readFileSync('src-tauri/Cargo.toml', 'utf8')

const packageSection = cargoToml.match(/\[package\]([\s\S]*?)(?:\n\[|$)/)
if (!packageSection) {
  throw new Error('src-tauri/Cargo.toml is missing a [package] section')
}
const cargoVersionMatch = packageSection[1].match(/^version\s*=\s*"([^"]+)"\s*$/m)
if (!cargoVersionMatch) {
  throw new Error('src-tauri/Cargo.toml [package] is missing a version')
}

const versions = {
  packageJson: packageJson.version,
  cargo: cargoVersionMatch[1],
  tauri: tauriConfig.version,
}

for (const [source, version] of Object.entries(versions)) {
  if (version !== expectedVersion) {
    throw new Error(`${source} version is ${String(version)}, expected reserved Desktop release version ${expectedVersion}`)
  }
}

if (new Set(Object.values(versions)).size !== 1) {
  throw new Error(`Desktop version mismatch: ${JSON.stringify(versions)}`)
}

console.log(`Desktop release version gate PASS: ${expectedVersion}`)
