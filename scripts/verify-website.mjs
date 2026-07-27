import { readFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import path from 'node:path'

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
const read = (relative) => readFile(path.join(root, relative), 'utf8')
const [html, script, styles, headers, readme] = await Promise.all([
  read('website/index.html'),
  read('website/main.js'),
  read('website/styles.css'),
  read('website/_headers'),
  read('README.md'),
])

function assert(condition, message) {
  if (!condition) throw new Error(`Website verification failed: ${message}`)
}

function attributeValues(source, attribute) {
  return [...source.matchAll(new RegExp(`\\b${attribute}=["']([^"']+)["']`, 'g'))]
    .map((match) => match[1])
}

const maintainedCopy = `${html}\n${script}\n${readme}`
const forbidden = [
  [/releases\/download\/v\d+/i, 'hard-coded version download URL'],
  [/\biTime_\d+\.\d+\.\d+_x64-setup\.exe\b/i, 'hard-coded installer filename'],
  [/\b[0-9a-f]{64}\b/i, 'hard-coded SHA-256'],
  [/\bKeyStats\b/i, 'obsolete KeyStats claim'],
  [/开源/, 'unlicensed open-source claim'],
  [/\bv0\.1\.0\b/i, 'hard-coded previous version'],
]
for (const [pattern, label] of forbidden) {
  assert(!pattern.test(maintainedCopy), label)
}

const canonical = html.match(/<link\b[^>]*rel=["']canonical["'][^>]*href=["']([^"']+)["']/i)?.[1]
const ogUrl = html.match(/<meta\b[^>]*property=["']og:url["'][^>]*content=["']([^"']+)["']/i)?.[1]
const ogImage = html.match(/<meta\b[^>]*property=["']og:image["'][^>]*content=["']([^"']+)["']/i)?.[1]
for (const [name, value] of [['canonical', canonical], ['og:url', ogUrl], ['og:image', ogImage]]) {
  assert(value?.startsWith('https://'), `${name} must be an absolute HTTPS URL`)
}

const ids = new Set(attributeValues(html, 'id'))
for (const href of attributeValues(html, 'href').filter((value) => value.startsWith('#'))) {
  assert(href.length > 1 && ids.has(decodeURIComponent(href.slice(1))), `missing anchor target ${href}`)
}

for (const value of [...attributeValues(html, 'href'), ...attributeValues(html, 'src')]) {
  if (/^https?:/i.test(value)) {
    assert(value.startsWith('https://'), `external resource must use HTTPS: ${value}`)
  }
}

for (const source of attributeValues(html, 'src').filter((value) => value.startsWith('./'))) {
  await readFile(path.join(root, 'website', source))
}

for (const name of ['home', 'ai', 'timeline', 'weekly', 'input']) {
  const [baseline, websiteAsset] = await Promise.all([
    readFile(path.join(root, 'tests', 'visual', 'baseline', `wide-${name}.png`)),
    readFile(path.join(root, 'website', 'assets', 'screenshots', `${name}.png`)),
  ])
  assert(baseline.equals(websiteAsset), `website screenshot ${name}.png is not the verified wide baseline`)
}

for (const role of ['installer', 'portable']) {
  for (const attribute of ['data-release-file', 'data-release-size', 'data-release-link', 'data-release-sha', 'data-copy-role']) {
    assert(html.includes(`${attribute}="${role}"`), `missing ${attribute}="${role}"`)
  }
}

for (const token of [
  'https://api.github.com/repos/lingcang728/iTime/releases/latest',
  'asset.digest',
  '^sha256:[0-9a-f]{64}$',
  'asset.browser_download_url',
  'asset.name === `iTime_${version}_x64-setup.exe`',
  "event.key === 'Escape'",
  "addEventListener('pointerdown'",
]) {
  assert(script.includes(token), `release/menu runtime is missing ${token}`)
}
assert(script.includes('release.assets.find'), 'release assets must be selected from API data')
assert(script.includes('button.disabled = false'), 'copy buttons must remain disabled until digest validation')
assert(styles.includes('.copy-btn:disabled'), 'disabled copy state must be visible')
assert(!/\bimmutable\b/i.test(headers), 'fixed-name assets must not use immutable caching')

console.log('Website verification passed: release metadata, links, anchors, privacy copy, menu behavior, and caching.')
