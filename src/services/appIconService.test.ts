import { beforeEach, describe, expect, it, vi } from 'vitest'

vi.mock('../platform/desktop', () => ({
  isTauriRuntime: () => false,
  listenDesktop: async () => () => undefined,
}))

import {
  clearAppIconMemory,
  peekAppIcon,
  resolveAppIcon,
} from './appIconService'

describe('appIconService cache dimensions', () => {
  beforeEach(() => {
    clearAppIconMemory()
  })

  it('keeps multiple requested sizes for one identity independent', async () => {
    const small = await resolveAppIcon({ appIdentity: 'app:shared-size-test', requestedSize: 32 })
    const large = await resolveAppIcon({ appIdentity: 'app:shared-size-test', requestedSize: 128 })

    expect(small.appIdentity).toBe(large.appIdentity)
    expect(small.width).toBe(32)
    expect(large.width).toBe(128)
    expect(peekAppIcon(small.appIdentity, 32)?.width).toBe(32)
    expect(peekAppIcon(small.appIdentity, 128)?.width).toBe(128)
  })

  it('uses the same cache entry for sizes that clamp to one supported size', async () => {
    const result = await resolveAppIcon({ appIdentity: 'app:clamped-size-test', requestedSize: 1_024 })

    expect(result.width).toBe(256)
    expect(peekAppIcon(result.appIdentity, 1_024)).toBe(result)
    expect(peekAppIcon(result.appIdentity, 256)).toBe(result)
  })
})
