import fs from 'node:fs'
import path from 'node:path'
import pixelmatch from 'pixelmatch'
import { PNG } from 'pngjs'
import { ssim } from 'ssim.js'

const root = process.cwd()
const output = path.join(root, 'artifacts', 'visual')
const baseline = path.join(root, 'tests', 'visual', 'baseline')
const referencePath = path.join(root, 'iTime原型图.png')
const skipReference = process.argv.includes('--skip-reference')
const referencePages = ['home', 'ai', 'weekly']
const darkPages = ['home', 'ai', 'timeline', 'input', 'weekly', 'goals', 'settings']
const report = { thresholds: { referenceSsim: 0.90, regressionStructureSsim: 0.985, structureRadius: 14, anchorCssPx: 4, regressionAnchorCssPx: 2, colorThreshold: 0.12 }, reference: [], regression: [], coverage: {} }
const read = (file) => PNG.sync.read(fs.readFileSync(file))

function resize(source, width, height) {
  const target = new PNG({ width, height })
  for (let y = 0; y < height; y += 1) {
    const sy = Math.min(source.height - 1, Math.floor(y * source.height / height))
    for (let x = 0; x < width; x += 1) {
      const sx = Math.min(source.width - 1, Math.floor(x * source.width / width))
      const from = (sy * source.width + sx) * 4
      const to = (y * width + x) * 4
      source.data.copy(target.data, to, from, from + 4)
    }
  }
  return target
}

function crop(source, rect) {
  const target = new PNG({ width: rect.width, height: rect.height })
  PNG.bitblt(source, target, rect.x, rect.y, rect.width, rect.height, 0, 0)
  return target
}

function structuralBlur(source, radius) {
  const { width, height } = source
  const gray = new Float64Array(width * height)
  const horizontal = new Float64Array(width * height)
  const target = new PNG({ width, height })
  for (let index = 0; index < width * height; index += 1) {
    gray[index] = source.data[index * 4] * 0.2126 + source.data[index * 4 + 1] * 0.7152 + source.data[index * 4 + 2] * 0.0722
  }
  for (let y = 0; y < height; y += 1) {
    let sum = 0
    for (let x = -radius; x <= radius; x += 1) sum += gray[y * width + Math.max(0, Math.min(width - 1, x))]
    for (let x = 0; x < width; x += 1) {
      horizontal[y * width + x] = sum / (radius * 2 + 1)
      sum += gray[y * width + Math.min(width - 1, x + radius + 1)] - gray[y * width + Math.max(0, x - radius)]
    }
  }
  for (let x = 0; x < width; x += 1) {
    let sum = 0
    for (let y = -radius; y <= radius; y += 1) sum += horizontal[Math.max(0, Math.min(height - 1, y)) * width + x]
    for (let y = 0; y < height; y += 1) {
      const value = Math.round(sum / (radius * 2 + 1))
      const index = (y * width + x) * 4
      target.data[index] = value
      target.data[index + 1] = value
      target.data[index + 2] = value
      target.data[index + 3] = 255
      sum += horizontal[Math.min(height - 1, y + radius + 1) * width + x] - horizontal[Math.max(0, y - radius) * width + x]
    }
  }
  return target
}

function edgeAnchor(image) {
  let continuity = 0
  let header = 0
  const searchStart = image.width >= 900 ? 140 : 90
  const searchEnd = image.width >= 900 ? 210 : 145
  for (let x = searchStart; x < Math.min(searchEnd, image.width - 1); x += 1) {
    let score = 0
    for (let y = 30; y < image.height - 30; y += 1) {
      const a = (y * image.width + x - 1) * 4
      const b = (y * image.width + x) * 4
      const difference = Math.abs(image.data[a] - image.data[b]) + Math.abs(image.data[a + 1] - image.data[b + 1]) + Math.abs(image.data[a + 2] - image.data[b + 2])
      if (difference > 3) score += 1
    }
    if (score > continuity) { continuity = score; header = x }
  }
  return header
}

function comparePair(actual, expected, name, threshold) {
  const diff = new PNG({ width: actual.width, height: actual.height })
  const pixels = pixelmatch(actual.data, expected.data, diff.data, actual.width, actual.height, { threshold })
  const ratio = pixels / (actual.width * actual.height)
  fs.writeFileSync(path.join(output, `diff-${name}.png`), PNG.sync.write(diff))
  return { name, differingPixels: pixels, pixelRatio: ratio, ssim: ssim(expected, actual, { ssim: 'fast' }).mssim }
}

if (!skipReference) {
  if (!fs.existsSync(referencePath)) {
    for (const page of referencePages) report.reference.push({ name: `reference-${page}`, missingExpected: true })
  } else {
    const composite = read(referencePath)
    const rects = {
      home: { x: 16, y: 16, width: 560, height: 900 },
      ai: { x: 587, y: 16, width: 545, height: 900 },
      weekly: { x: 1144, y: 16, width: 512, height: 900 },
    }
    for (const [page, rect] of Object.entries(rects)) {
      const actualPath = path.join(output, `reference-${page}.png`)
      if (!fs.existsSync(actualPath)) {
        report.reference.push({ name: `reference-${page}`, missingActual: true })
        continue
      }
      const actual = read(actualPath)
      const expected = resize(crop(composite, rect), actual.width, actual.height)
      const result = comparePair(actual, expected, `reference-${page}`, 0.12)
      result.rawSsim = result.ssim
      result.ssim = ssim(structuralBlur(expected, report.thresholds.structureRadius), structuralBlur(actual, report.thresholds.structureRadius), { ssim: 'fast', downsample: false }).mssim
      result.anchorDelta = Math.abs(edgeAnchor(actual) - edgeAnchor(expected))
      report.reference.push(result)
    }
  }
}

if (fs.existsSync(baseline)) {
  for (const file of fs.readdirSync(baseline).filter((value) => value.endsWith('.png')).sort()) {
    const actualPath = path.join(output, file)
    if (!fs.existsSync(actualPath)) {
      report.regression.push({ name: file, missingActual: true })
      continue
    }
    const actual = read(actualPath)
    const expected = read(path.join(baseline, file))
    if (actual.width !== expected.width || actual.height !== expected.height) {
      report.regression.push({ name: file, pixelRatio: 1, sizeMismatch: true })
      continue
    }
    const result = comparePair(actual, expected, `baseline-${path.basename(file, '.png')}`, 0.12)
    result.rawSsim = result.ssim
    result.ssim = ssim(structuralBlur(expected, report.thresholds.structureRadius), structuralBlur(actual, report.thresholds.structureRadius), { ssim: 'fast', downsample: false }).mssim
    result.anchorDelta = Math.abs(edgeAnchor(actual) - edgeAnchor(expected))
    report.regression.push(result)
  }
}

const baselineFiles = new Set(fs.existsSync(baseline) ? fs.readdirSync(baseline) : [])
for (const page of darkPages) {
  const file = `dark-${page}.png`
  if (!baselineFiles.has(file)) report.regression.push({ name: file, missingExpected: true })
}

const referenceCompared = report.reference.filter((item) => !item.missingExpected && !item.missingActual && !item.sizeMismatch).length
const darkCompared = report.regression.filter((item) => item.name.startsWith('baseline-dark-') && !item.missingExpected && !item.missingActual && !item.sizeMismatch).length
report.coverage = {
  reference: { expected: skipReference ? 0 : referencePages.length, compared: referenceCompared, skipped: skipReference },
  darkRegression: { expected: darkPages.length, compared: darkCompared },
}

fs.writeFileSync(path.join(output, 'report.json'), JSON.stringify(report, null, 2))
const referenceFailed = report.reference.some((item) => item.missingExpected || item.missingActual || item.sizeMismatch || item.ssim < report.thresholds.referenceSsim || item.anchorDelta > report.thresholds.anchorCssPx)
  || (!skipReference && referenceCompared !== referencePages.length)
const regressionFailed = report.regression.some((item) => item.missingExpected || item.missingActual || item.sizeMismatch || item.ssim < report.thresholds.regressionStructureSsim || item.anchorDelta > report.thresholds.regressionAnchorCssPx)
  || darkCompared !== darkPages.length
console.log(JSON.stringify({ coverage: report.coverage, reference: report.reference, regression: report.regression, passed: !referenceFailed && !regressionFailed }, null, 2))
process.exit(referenceFailed || regressionFailed ? 1 : 0)
