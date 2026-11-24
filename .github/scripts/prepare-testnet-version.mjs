import fs from 'node:fs'

const STABLE = '0.1.0'
const TESTNET = '0.1.0-testnet.1'
const STABLE_ID = 'io.aurialabs.pulsedag.desktop'
const TESTNET_ID = 'io.aurialabs.pulsedag.desktop.testnet'

const pkgPath = 'package.json'
const cargoPath = 'src-tauri/Cargo.toml'
const baseConfigPath = 'src-tauri/tauri.conf.json'
const overlayPath = 'src-tauri/tauri.testnet.conf.json'

function ensure(condition, message) {
  if (!condition) throw new Error(`Testnet prerelease gate: ${message}`)
}

function cargoPackageVersion(toml) {
  const pkg = toml.match(/\[package\]([\s\S]*?)(?=\n\[|$)/)
  ensure(pkg, 'missing Cargo [package] section')
  const version = pkg[1].match(/^version\s*=\s*"([^"]+)"\s*$/m)
  ensure(version, 'missing Cargo [package].version')
  return version[1]
}

const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf8'))
const cargo = fs.readFileSync(cargoPath, 'utf8')
const base = JSON.parse(fs.readFileSync(baseConfigPath, 'utf8'))
const testnet = JSON.parse(fs.readFileSync(overlayPath, 'utf8'))

function checkOverlay() {
  ensure(base.version === STABLE, 'stable Tauri config version must remain 0.1.0')
  ensure(base.identifier === STABLE_ID, 'stable OS identifier must remain unchanged')
  ensure(base.productName === 'PulseDAG Desktop', 'stable product name must remain unchanged')
  ensure(testnet.version === TESTNET, `testnet overlay version must be ${TESTNET}`)
  ensure(testnet.identifier === TESTNET_ID && testnet.identifier !== base.identifier,
    'testnet OS identifier must be distinct')
  ensure(testnet.productName === 'PulseDAG Desktop Testnet',
    'testnet product name must visibly differ')
  ensure(testnet.app?.windows?.some(window =>
    typeof window.title === 'string' &&
    window.title.includes('Testnet') &&
    window.title.includes('PREVIEW ONLY')),
    'testnet window must identify itself as a preview')
  ensure(typeof testnet.bundle?.longDescription === 'string'
    && testnet.bundle.longDescription.includes('No public v3.0 testnet GO'),
    'testnet bundle must disclaim absent GO')
}

function checkVersion(version) {
  ensure(pkg.version === version, `package.json expected ${version}, got ${pkg.version}`)
  ensure(cargoPackageVersion(cargo) === version,
    `Cargo [package] version must be ${version}`)
}

const operation = process.argv[2] ?? 'check'
checkOverlay()

if (operation === 'check') {
  checkVersion(STABLE)
  console.log(`Testnet channel metadata PASS: stable ${STABLE}, reserved testnet ${TESTNET}`)
} else if (operation === 'prepare') {
  checkVersion(STABLE)
  // This operation is for an ephemeral checkout only. Source control retains
  // the stable v0.1.0 metadata and never changes the mainnet installer identity.
  pkg.version = TESTNET
  fs.writeFileSync(pkgPath, `${JSON.stringify(pkg, null, 2)}\n`)
  const newCargo = cargo.replace(
    /(\[package\][\s\S]*?^version\s*=\s*")[^"]+(")/m,
    (_match, before, after) => `${before}${TESTNET}${after}`,
  )
  ensure(newCargo !== cargo, 'could not rewrite Cargo package version')
  fs.writeFileSync(cargoPath, newCargo)
  console.log(`Prepared ephemeral testnet working tree ${TESTNET}; no GO or public release granted`)
} else if (operation === 'verify-prepared') {
  checkVersion(TESTNET)
  console.log(`Testnet bundle metadata PASS: ${TESTNET}, separate identifier ${TESTNET_ID}`)
} else {
  throw new Error('Choose check, prepare, or verify-prepared')
}
