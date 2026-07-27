import { describe, expect, it, vi } from 'vitest'
import { changeAutostartState, queryAutostartState } from './autostart'

describe('autostart registration boundaries', () => {
  it('refreshes status with one read and no writes', async () => {
    const isEnabled = vi.fn(async () => true)
    expect(await queryAutostartState(isEnabled)).toBe(true)
    expect(isEnabled).toHaveBeenCalledTimes(1)
  })

  it.each([
    [true, 'enable'],
    [false, 'disable'],
  ] as const)('writes only for an explicit %s request', async (enabled, expectedWrite) => {
    const api = {
      enable: vi.fn(async () => undefined),
      disable: vi.fn(async () => undefined),
      isEnabled: vi.fn(async () => enabled),
    }
    expect(await changeAutostartState(enabled, api)).toBe(enabled)
    expect(api[expectedWrite]).toHaveBeenCalledTimes(1)
    expect(api[expectedWrite === 'enable' ? 'disable' : 'enable']).not.toHaveBeenCalled()
    expect(api.isEnabled).toHaveBeenCalledTimes(1)
  })
})
