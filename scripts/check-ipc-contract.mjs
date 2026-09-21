// IPC contract drift check: statically compares the Rust serialization surface
// (serde structs/enums behind #[tauri::command] and .emit payloads) with the
// frontend zod schemas / TypeScript interfaces that parse them.
//
// Direction rules:
//   - a key the frontend expects but Rust never sends            -> error
//   - a serialized Rust key the frontend ignores (zod strips)    -> warning
//   - enum variant sets must match exactly                       -> error
//   - command/event names referenced by the frontend must exist  -> error
//   - manifest "literals" pin constant wire values (e.g. contentCaptured:false)
//
// Manifest: tests/contract/ipc-contract.json
import fs from 'node:fs'
import path from 'node:path'

const root = process.cwd()
const manifestPath = path.join(root, 'tests', 'contract', 'ipc-contract.json')
const manifest = JSON.parse(fs.readFileSync(manifestPath, 'utf8'))

const errors = []
const warnings = []

/* ---------- generic helpers ---------- */

function* walk(dir) {
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name)
    if (entry.isDirectory()) yield* walk(full)
    else yield full
  }
}

function matchBraces(text, openIndex) {
  // openIndex points at '{' — returns index of the matching '}'.
  let depth = 0
  for (let i = openIndex; i < text.length; i += 1) {
    const ch = text[i]
    if (ch === '{') depth += 1
    else if (ch === '}') {
      depth -= 1
      if (depth === 0) return i
    }
  }
  return -1
}

function splitTopLevel(text, separators = ',;\n') {
  const parts = []
  let depth = 0
  let current = ''
  for (const ch of text) {
    if ('{[(<'.includes(ch)) depth += 1
    if ('}])>'.includes(ch)) depth -= 1
    if (separators.includes(ch) && depth === 0) {
      parts.push(current)
      current = ''
    } else {
      current += ch
    }
  }
  if (current.trim()) parts.push(current)
  return parts
}

function snakeToCamel(name) {
  return name.replace(/_([a-z0-9])/g, (_, c) => c.toUpperCase())
}

function variantToCamel(name) {
  // serde rename_all="camelCase" maps PascalCase enum variants to lowerCamel.
  const camel = snakeToCamel(name)
  return camel.charAt(0).toLowerCase() + camel.slice(1)
}

function stripLineComments(text) {
  // Remove `//` comments that are outside string literals.
  let out = ''
  let quote = null
  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i]
    if (quote) {
      out += ch
      if (ch === quote && text[i - 1] !== '\\') quote = null
      continue
    }
    if (ch === '"' || ch === "'" || ch === '`') { quote = ch; out += ch; continue }
    if (ch === '/' && text[i + 1] === '/') {
      while (i < text.length && text[i] !== '\n') i += 1
      out += '\n'
      continue
    }
    out += ch
  }
  return out
}

/* ---------- Rust parsing ---------- */

function stripRustTestModules(text) {
  // Remove `#[cfg(test)] ... mod xxx { ... }` blocks so test-only
  // initializers do not confuse the literal/field analysis.
  let out = ''
  let cursor = 0
  const pattern = /#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]/g
  let match
  while ((match = pattern.exec(text)) !== null) {
    const rest = text.slice(match.index)
    const modMatch = rest.match(/mod\s+\w+\s*\{/)
    if (!modMatch) continue
    const modStart = match.index + modMatch.index
    const braceStart = text.indexOf('{', modStart)
    const braceEnd = matchBraces(text, braceStart)
    if (braceEnd < 0) break
    out += text.slice(cursor, match.index)
    cursor = braceEnd + 1
  }
  out += text.slice(cursor)
  return out
}

function parseSerdeAttrs(text) {
  const attrs = {}
  const serdeAttr = /#\s*\[\s*serde\s*\(([^)]*)\)\s*\]/g
  let m
  while ((m = serdeAttr.exec(text)) !== null) {
    for (const part of splitTopLevel(m[1])) {
      const kv = part.match(/^\s*(\w+)\s*(?:=\s*"([^"]*)"|\s*=\s*(\w+)\s*\(.*\))?\s*$/) || part.match(/^\s*(\w+)\s*$/)
      if (kv) attrs[kv[1]] = kv[2] ?? true
    }
  }
  return attrs
}

function parseRustTypes(sources) {
  // sources: [{file, text (test-stripped)}]
  const types = new Map()
  const typePattern = /(pub(?:\([^)]*\))?\s+)?(struct|enum)\s+(\w+)\s*([^\{]*)\{/g
  for (const { file, text } of sources) {
    let match
    while ((match = typePattern.exec(text)) !== null) {
      const [, , kind, name] = match
      const braceStart = match.index + match[0].length - 1
      const braceEnd = matchBraces(text, braceStart)
      if (braceEnd < 0) continue
      const body = text.slice(braceStart + 1, braceEnd)
      // container serde attrs live in the ~400 chars before `struct/enum`
      const headerStart = Math.max(0, match.index - 400)
      const header = text.slice(headerStart, match.index)
      const containerAttrs = parseSerdeAttrs(header)
      const entry = { kind, name, file, attrs: containerAttrs, fields: [], variants: [] }
      if (kind === 'enum') {
        for (const part of splitTopLevel(body, ',')) {
          const vm = part.trim().match(/^(?:#\[[^\]]*\]\s*)*(\w+)/)
          if (vm && vm[1] !== 'pub') entry.variants.push(vm[1])
        }
      } else {
        // field pattern: optional attrs then `pub... name: Type`
        const fieldPattern = /((?:#\s*\[[^\]]*\]\s*)*)(?:pub(?:\([^)]*\))?\s+)?(\w+)\s*:\s*([^,\n]+)/g
        let fm
        while ((fm = fieldPattern.exec(body)) !== null) {
          const attrs = parseSerdeAttrs(fm[1])
          entry.fields.push({
            name: fm[2],
            type: fm[3].trim().replace(/,$/, ''),
            attrs,
          })
        }
      }
      types.set(name, entry)
    }
  }
  return types
}

function wireName(field, containerAttrs) {
  if (typeof field.attrs.rename === 'string') return field.attrs.rename
  const convention = containerAttrs.rename_all
  if (convention === 'camelCase') return snakeToCamel(field.name)
  return field.name
}

function serializedKeys(typeName, types, depth = 0) {
  // Returns the set of keys this type emits on the wire.
  const type = types.get(typeName)
  if (!type || type.kind !== 'struct' || depth > 4) return type ? new Set() : null
  const keys = new Set()
  for (const field of type.fields) {
    if (field.attrs.skip_serializing || field.attrs.skip) continue
    if (field.attrs.flatten) {
      const innerName = (field.type.match(/(\w+)\s*$/) || [])[1]
      const inner = innerName ? types.get(innerName) : null
      if (inner) {
        for (const key of serializedKeys(innerName, types, depth + 1)) keys.add(key)
        continue
      }
    }
    keys.add(wireName(field, type.attrs))
  }
  return keys
}

function deserializedFieldRules(typeName, types) {
  const type = types.get(typeName)
  if (!type || type.kind !== 'struct') return null
  const required = []
  const accepted = []
  for (const field of type.fields) {
    if (field.attrs.skip || field.attrs.skip_deserializing) continue
    const name = wireName(field, type.attrs)
    accepted.push(name)
    const optional = /^Option\s*</.test(field.type) || field.attrs.default !== undefined
    if (!optional) required.push(name)
  }
  return { required, accepted, denyUnknown: Boolean(type.attrs.deny_unknown_fields) }
}

/* ---------- TypeScript parsing ---------- */

function parseTsObjectKeys(text) {
  // Extracts top-level `key` entries of a `{ ... }` object/type literal.
  // Returns [{key, value, raw}]
  const inner = text.trim().replace(/^\{/, '').replace(/\}$/, '')
  const keys = []
  for (const part of splitTopLevel(inner)) {
    const m = part.match(/^\s*(?:readonly\s+)?["']?(\w+)["']?\s*\?\s*:/)
      || part.match(/^\s*(?:readonly\s+)?["']?(\w+)["']?\s*:/)
      || part.match(/^\s*(\w+)\s*$/)
    if (m) keys.push({ key: m[1], raw: part })
  }
  return keys
}

function extractZodNode(text) {
  // Classify a zod expression text: {kind, keys?, enum?, literal?}
  const trimmed = text.trim()
  const obj = trimmed.match(/z\.object\s*\(\s*\{/)
  if (obj) {
    const start = trimmed.indexOf('{', obj.index)
    const end = matchBraces(trimmed, start)
    const node = { kind: 'object', keys: new Map() }
    if (end > 0) {
      for (const part of splitTopLevel(trimmed.slice(start + 1, end))) {
        const km = part.match(/^\s*["']?(\w+)["']?\s*:\s*/)
        if (!km) continue
        const value = part.slice(km[0].length)
        node.keys.set(km[1], { raw: value, node: extractZodNode(value) })
      }
    }
    return node
  }
  const arr = trimmed.match(/z\.array\s*\(/)
  if (arr) {
    // find inner expression: between first '(' and its match — reuse brace
    // depth counting on parens via splitTopLevel-style scan
    let depth = 0
    const start = trimmed.indexOf('(', arr.index)
    let end = -1
    for (let i = start; i < trimmed.length; i += 1) {
      if (trimmed[i] === '(') depth += 1
      if (trimmed[i] === ')') { depth -= 1; if (depth === 0) { end = i; break } }
    }
    return { kind: 'array', element: extractZodNode(trimmed.slice(start + 1, end)) }
  }
  const en = trimmed.match(/z\.enum\s*\(\s*\[([^\]]*)\]/)
  if (en) {
    return { kind: 'enum', values: [...en[1].matchAll(/["']([^"']+)["']/g)].map((m) => m[1]) }
  }
  const lit = trimmed.match(/z\.literal\s*\(\s*([^)]*)\)/)
  if (lit) {
    const raw = lit[1].trim()
    let value = raw
    if (raw === 'true') value = true
    else if (raw === 'false') value = false
    else if (/^-?\d+$/.test(raw)) value = Number(raw)
    else value = raw.replace(/^["']|["']$/g, '')
    return { kind: 'literal', value }
  }
  return { kind: 'leaf', optional: /\.optional\s*\(/.test(trimmed), nullable: /\.nullable\s*\(/.test(trimmed) }
}

function parseTsSchemas(sources) {
  const schemas = new Map() // name -> {kind:'object', keys:Map} | {kind:'enum'...}
  for (const { text } of sources) {
    // const <name> = z.object({...}) / z.enum([...]) etc.
    const pattern = /(?:export\s+)?const\s+(\w+)\s*=\s*(z\.)/g
    let m
    while ((m = pattern.exec(text)) !== null) {
      // capture the expression up to a top-level statement end: take from m.index of 'z.' to
      // the first newline that is at brace/paren depth 0.
      const exprStart = m.index + m[0].length - 2
      let depth = 0
      let end = text.length
      for (let i = exprStart; i < text.length; i += 1) {
        const ch = text[i]
        if ('({['.includes(ch)) depth += 1
        if (')}]'.includes(ch)) depth -= 1
        if (ch === '\n' && depth === 0) { end = i; break }
      }
      // multi-line z.object bodies end at the closing `})` + optional `.refine(...)` chain
      // extend end while next line starts with `.`
      let rest = text.slice(end)
      const chain = rest.match(/^(\s*\.\s*\w+\s*\([^)]*\))*/)
      if (chain) end += chain[0].length
      schemas.set(m[1], extractZodNode(text.slice(exprStart, end)))
    }
    // interface X { ... }
    const iface = /(?:export\s+)?interface\s+(\w+)\s*\{/g
    while ((m = iface.exec(text)) !== null) {
      const braceStart = m.index + m[0].length - 1
      const braceEnd = matchBraces(text, braceStart)
      if (braceEnd < 0) continue
      const node = { kind: 'object', keys: new Map() }
      for (const { key, raw } of parseTsObjectKeys(text.slice(braceStart, braceEnd + 1))) {
        node.keys.set(key, { raw, node: { kind: 'leaf' } })
      }
      schemas.set(m[1], node)
    }
  }
  return schemas
}

function resolveTsPath(schemas, spec) {
  // spec: "file.ts#schemaName.nestedKey.nestedArrayKey[]"
  const [file, ref] = spec.split('#')
  const segments = ref.split('.')
  const rootName = segments.shift()
  const node = schemas.get(rootName)
  if (!node) return { error: `TS schema not found: ${ref} (in ${file})` }
  let current = node
  for (const segment of segments) {
    const isArray = segment.endsWith('[]')
    const key = isArray ? segment.slice(0, -2) : segment
    if (current.kind === 'array') current = current.element
    if (current.kind !== 'object' || !current.keys.has(key)) {
      return { error: `TS path segment missing: ${segment} in ${spec}` }
    }
    current = current.keys.get(key).node
    if (isArray && current.kind === 'array') current = current.element
  }
  return { node: current }
}

/* ---------- invoke / listen extraction ---------- */

function extractFrontendCalls(sources) {
  const invokes = [] // {name, generic, argKeys, file}
  const listens = []  // {name, payloadKeys, file}
  for (const { file, text } of sources) {
    const callPattern = /\b(invoke|invokeDesktop|listen|listenDesktop)\b/g
    let m
    while ((m = callPattern.exec(text)) !== null) {
      const fn = m[1]
      let i = m.index + fn.length
      // skip whitespace
      while (/\s/.test(text[i])) i += 1
      // optional generic <...>
      let generic = ''
      if (text[i] === '<') {
        let depth = 0
        const start = i
        for (; i < text.length; i += 1) {
          if (text[i] === '<') depth += 1
          else if (text[i] === '>') { depth -= 1; if (depth === 0) { i += 1; break } }
        }
        generic = text.slice(start, i)
      }
      while (/\s/.test(text[i])) i += 1
      if (text[i] !== '(') continue
      const parenStart = i
      let depth = 0
      let parenEnd = -1
      for (let j = parenStart; j < text.length; j += 1) {
        if (text[j] === '(') depth += 1
        else if (text[j] === ')') { depth -= 1; if (depth === 0) { parenEnd = j; break } }
      }
      if (parenEnd < 0) continue
      const argsText = text.slice(parenStart + 1, parenEnd)
      const nameMatch = argsText.match(/^\s*['"]([\w:|-]+)['"]/)
      if (!nameMatch) continue
      const name = nameMatch[1]
      const rest = argsText.slice(nameMatch[0].length).replace(/^\s*,/, '')
      let argKeys = null
      if (rest.trim().startsWith('{')) {
        argKeys = parseTsObjectKeys(rest.trim()).map((k) => ({ key: k.key, raw: k.raw }))
      }
      let payloadKeys = null
      const genericObj = generic.match(/\{([\s\S]*)\}/)
      if (genericObj) payloadKeys = parseTsObjectKeys(genericObj[0]).map((k) => k.key)
      const record = { name, file, argKeys, payloadKeys, generic }
      if (fn.startsWith('invoke')) invokes.push(record)
      else listens.push(record)
    }
  }
  return { invokes, listens }
}

/* ---------- collect sources ---------- */

const rustSources = [...walk(path.join(root, 'src-tauri', 'src'))]
  .filter((file) => file.endsWith('.rs'))
  .map((file) => ({ file, text: stripRustTestModules(fs.readFileSync(file, 'utf8')) }))
const tsSources = [...walk(path.join(root, 'src'))]
  .filter((file) => (file.endsWith('.ts') || file.endsWith('.vue')) && !file.endsWith('.test.ts'))
  .map((file) => ({ file: path.relative(root, file), text: stripLineComments(fs.readFileSync(file, 'utf8')) }))

const types = parseRustTypes(rustSources)
const schemas = parseTsSchemas(tsSources)
const { invokes, listens } = extractFrontendCalls(tsSources)

// commands registered in generate_handler![...]
const handlerSets = []
for (const { text } of rustSources) {
  const pattern = /generate_handler!\s*\[([^\]]*)\]/g
  let m
  while ((m = pattern.exec(text)) !== null) {
    for (const part of splitTopLevel(m[1])) {
      const name = part.trim().split('::').pop().trim()
      if (name) handlerSets.push(name)
    }
  }
}
const registeredCommands = new Set(handlerSets)

// #[tauri::command] fn signatures -> expected camelCase arg names
const commandArgs = new Map()
for (const { text } of rustSources) {
  const pattern = /#\s*\[\s*tauri::command\s*\]\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)\s*\(([^)]*)\)/g
  let m
  while ((m = pattern.exec(text)) !== null) {
    const params = []
    for (const part of splitTopLevel(m[2])) {
      const pm = part.match(/(\w+)\s*:\s*(.+)/)
      if (!pm) continue
      const type = pm[2].trim()
      if (/^(State|AppHandle|Window|Manager|Runtime|Emitter|WebviewWindow)\b/.test(type) || type.includes('State<') || type.includes('AppHandle')) continue
      params.push(snakeToCamel(pm[1]))
    }
    commandArgs.set(m[1], params)
  }
}

// emitted event names
const emittedEvents = new Set()
for (const { text } of rustSources) {
  const pattern = /\.emit(?:_all|_to)?\s*\(\s*"([^"]+)"/g
  let m
  while ((m = pattern.exec(text)) !== null) emittedEvents.add(m[1])
}

/* ---------- checks ---------- */

// 1. struct <-> schema key comparison
for (const pair of manifest.responses ?? []) {
  const rustKeys = serializedKeys(pair.rust, types)
  if (!rustKeys) { errors.push(`Rust type not found: ${pair.rust} (${pair.ts})`); continue }
  const resolved = resolveTsPath(schemas, pair.ts)
  if (resolved.error) { errors.push(resolved.error); continue }
  const tsNode = resolved.node
  if (tsNode.kind !== 'object') { errors.push(`TS side is not an object schema: ${pair.ts}`); continue }
  const tsKeys = new Set(tsNode.keys.keys())
  const extraOk = new Set(pair.extraOk ?? [])
  for (const key of tsKeys) {
    if (!rustKeys.has(key)) errors.push(`${pair.ts}: frontend expects '${key}' but ${pair.rust} never serializes it`)
  }
  for (const key of rustKeys) {
    if (!tsKeys.has(key) && !extraOk.has(key)) warnings.push(`${pair.rust} serializes '${key}' but ${pair.ts} does not consume it (stripped by zod)`)
  }
}

// 2. enum variant comparison
for (const pair of manifest.enums ?? []) {
  const type = types.get(pair.rust)
  if (!type || type.kind !== 'enum') { errors.push(`Rust enum not found: ${pair.rust}`); continue }
  const resolved = resolveTsPath(schemas, pair.ts)
  if (resolved.error) { errors.push(resolved.error); continue }
  if (resolved.node.kind !== 'enum') { errors.push(`TS side is not a z.enum: ${pair.ts}`); continue }
  const convention = type.attrs.rename_all
  const rustValues = new Set(type.variants.map((v) => (convention === 'camelCase' ? variantToCamel(v) : convention === 'snake_case' ? v.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`).replace(/^_/, '') : v)))
  const tsValues = new Set(resolved.node.values)
  for (const value of tsValues) {
    if (!rustValues.has(value)) errors.push(`${pair.ts}: frontend z.enum value '${value}' not produced by ${pair.rust}`)
  }
  for (const value of rustValues) {
    if (!tsValues.has(value)) errors.push(`${pair.rust} enum variant '${value}' has no frontend counterpart in ${pair.ts} (closed z.enum would fail at runtime)`)
  }
}

// 3. constant wire-value guards (privacy literals like contentCaptured: false)
for (const guard of manifest.literals ?? []) {
  const type = types.get(guard.rust)
  if (!type) { errors.push(`Rust type not found for literal guard: ${guard.rust}`); continue }
  // find `TypeName { ... }` initializer blocks in non-test Rust code
  let initializers = 0
  const initPattern = new RegExp(`\\b${guard.rust}\\s*\\{`, 'g')
  const declPattern = /\b(?:struct|enum|impl|trait|type|fn|mod|let|const|static)\s*$/
  for (const { file, text } of rustSources) {
    let m
    while ((m = initPattern.exec(text)) !== null) {
      // skip declarations like `struct X {` — only `X { ... }` initializers count
      if (declPattern.test(text.slice(Math.max(0, m.index - 40), m.index))) continue
      const braceStart = m.index + m[0].length - 1
      const braceEnd = matchBraces(text, braceStart)
      if (braceEnd < 0) continue
      initializers += 1
      const body = text.slice(braceStart + 1, braceEnd)
      for (const [key, expected] of Object.entries(guard.fields)) {
        // field may be written as snake_case in Rust source
        const snake = key.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`)
        const fieldPattern = new RegExp(`\\b${snake}\\s*:\\s*(true|false|"[^"]*")`)
        const fm = body.match(fieldPattern)
        const expectedText = typeof expected === 'string' ? `"${expected}"` : String(expected)
        if (!fm || fm[1] !== expectedText) {
          errors.push(`${guard.rust} initializer in ${file} sets '${key}' to ${fm ? fm[1] : '<absent>'}, expected ${expectedText}`)
        }
      }
    }
  }
  if (initializers === 0) warnings.push(`no non-test initializer found for ${guard.rust}; literal guard could not be evaluated`)
}

// 4. command names + argument keys
const frontendCommands = invokes.filter((call) => !call.name.startsWith('plugin:'))
for (const call of frontendCommands) {
  if (!registeredCommands.has(call.name)) {
    errors.push(`${call.file}: invoke('${call.name}') has no matching #[tauri::command] in generate_handler!`)
    continue
  }
  const params = commandArgs.get(call.name)
  if (params && call.argKeys) {
    for (const { key } of call.argKeys) {
      if (!params.includes(key)) errors.push(`${call.file}: invoke('${call.name}') passes arg '${key}' which is not a camelCase param of the Rust command`)
    }
    for (const param of params) {
      if (!call.argKeys.some((k) => k.key === param)) warnings.push(`${call.file}: Rust command '${call.name}' param '${param}' is never sent by the frontend call site`)
    }
  }
}
for (const command of registeredCommands) {
  if (!frontendCommands.some((call) => call.name === command)) {
    warnings.push(`Rust command '${command}' is registered but never invoked from src/ (may be smoke-only)`)
  }
}

// 4b. inline response generics: invoke<{ a: T; b: T }>('cmd') key subset of Rust wire keys
for (const pair of manifest.commandResponses ?? []) {
  const rustKeys = serializedKeys(pair.rust, types)
  if (!rustKeys) { errors.push(`commandResponses: Rust type not found: ${pair.rust}`); continue }
  for (const call of frontendCommands.filter((c) => c.name === pair.command)) {
    for (const key of call.payloadKeys ?? []) {
      if (!rustKeys.has(key)) errors.push(`${call.file}: invoke('${pair.command}') reads '${key}' which ${pair.rust} never serializes`)
    }
  }
}

// 5. nested request payloads (e.g. resolve_app_icon's `request` arg)
for (const req of manifest.requests ?? []) {
  const call = frontendCommands.find((c) => c.name === req.command)
  if (!call || !call.argKeys) { errors.push(`request check: no invoke('${req.command}') with object args found`); continue }
  const arg = call.argKeys.find((k) => k.key === req.arg)
  if (!arg) { errors.push(`request check: invoke('${req.command}') lacks '${req.arg}' arg`); continue }
  const rules = deserializedFieldRules(req.rust, types)
  if (!rules) { errors.push(`request check: Rust type not found: ${req.rust}`); continue }
  const nestedKeys = parseTsObjectKeys(arg.raw.slice(arg.raw.indexOf(':') + 1)).map((k) => k.key)
  for (const key of nestedKeys) {
    if (!rules.accepted.includes(key)) {
      const severity = rules.denyUnknown ? 'error' : 'warning'
      ;(severity === 'error' ? errors : warnings).push(`request ${req.command}.${req.arg}: frontend sends '${key}' not present in ${req.rust}`)
    }
  }
  for (const required of rules.required) {
    if (!nestedKeys.includes(required)) errors.push(`request ${req.command}.${req.arg}: required field '${required}' of ${req.rust} is never sent`)
  }
}

// 6. event names + event payload keys
for (const call of listens) {
  if (!emittedEvents.has(call.name)) {
    errors.push(`${call.file}: listen*('${call.name}') has no matching Rust .emit(\"${call.name}\")`)
  }
}
for (const name of emittedEvents) {
  if (!listens.some((call) => call.name === name)) {
    warnings.push(`Rust emits '${name}' but nothing in src/ listens for it`)
  }
}
for (const pair of manifest.eventPayloads ?? []) {
  const type = types.get(pair.rust)
  if (!type) { errors.push(`event payload Rust type not found: ${pair.rust}`); continue }
  const rustKeys = serializedKeys(pair.rust, types)
  const call = listens.find((c) => c.name === pair.event)
  if (!call) { errors.push(`event '${pair.event}' has no frontend listener`); continue }
  if (call.payloadKeys) {
    for (const key of call.payloadKeys) {
      if (!rustKeys.has(key)) errors.push(`event '${pair.event}': frontend payload key '${key}' not serialized by ${pair.rust}`)
    }
  }
}

/* ---------- report ---------- */

for (const warning of warnings) console.warn(`warn  ${warning}`)
for (const error of errors) console.error(`fail  ${error}`)
console.log(`ipc-contract: ${errors.length} error(s), ${warnings.length} warning(s); ${registeredCommands.size} commands, ${emittedEvents.size} events, ${manifest.responses?.length ?? 0} response pairs checked`)
process.exit(errors.length ? 1 : 0)
