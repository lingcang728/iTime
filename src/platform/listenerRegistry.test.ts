import { describe, expect, it, vi } from 'vitest'
import { registerListenersIndependently } from './listenerRegistry'

describe('desktop listener registration', () => {
  it('keeps successful listeners when one registration fails', async () => {
    const firstCleanup = vi.fn()
    const thirdCleanup = vi.fn()
    const onError = vi.fn()
    const cleanups = await registerListenersIndependently([
      async () => firstCleanup,
      async () => { throw new Error('listener unavailable') },
      async () => thirdCleanup,
    ], onError)

    expect(cleanups).toEqual([firstCleanup, thirdCleanup])
    expect(onError).toHaveBeenCalledTimes(1)
  })
})
