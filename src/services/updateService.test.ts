import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  checkForDesktopUpdate,
  downloadAndInstallDesktopUpdate,
  resetUpdateServiceForTests,
  updateState,
} from './updateService'

const mocks = vi.hoisted(() => ({
  check: vi.fn(),
  downloadAndInstall: vi.fn(),
  getVersion: vi.fn(),
  invoke: vi.fn(),
  relaunch: vi.fn(),
}))

vi.mock('@tauri-apps/api/app', () => ({ getVersion: mocks.getVersion }))
vi.mock('@tauri-apps/api/core', () => ({ invoke: mocks.invoke }))
vi.mock('@tauri-apps/plugin-process', () => ({ relaunch: mocks.relaunch }))
vi.mock('@tauri-apps/plugin-updater', () => ({ check: mocks.check }))

describe('updateService', () => {
  beforeEach(() => {
    Object.defineProperty(window, '__TAURI_INTERNALS__', { value: {}, configurable: true })
    resetUpdateServiceForTests()
    vi.clearAllMocks()
    mocks.getVersion.mockResolvedValue('0.2.1')
    mocks.invoke.mockImplementation(async (command) => (
      command === 'prepare_for_update' ? { portable: false } : undefined
    ))
    mocks.relaunch.mockResolvedValue(undefined)
  })

  it('exposes release details and tracks download progress', async () => {
    mocks.downloadAndInstall.mockImplementation(async (onEvent) => {
      onEvent({ event: 'Started', data: { contentLength: 1_000 } })
      onEvent({ event: 'Progress', data: { chunkLength: 400 } })
      onEvent({ event: 'Finished' })
    })
    mocks.check.mockResolvedValue({
      currentVersion: '0.2.1',
      version: '0.2.2',
      date: '2026-07-29T00:00:00Z',
      body: '修复本地图标',
      rawJson: { size: 1_000 },
      downloadAndInstall: mocks.downloadAndInstall,
    })

    await checkForDesktopUpdate(true)
    expect(updateState).toMatchObject({
      status: 'available',
      currentVersion: '0.2.1',
      version: '0.2.2',
      notes: '修复本地图标',
      sizeBytes: 1_000,
    })

    await downloadAndInstallDesktopUpdate()
    expect(mocks.invoke).toHaveBeenCalledWith('prepare_for_update')
    expect(updateState.downloadedBytes).toBe(400)
    expect(updateState.status).toBe('installing')
    expect(mocks.relaunch).toHaveBeenCalledOnce()
  })

  it('restores collection if download fails after preparation', async () => {
    mocks.downloadAndInstall.mockRejectedValue(new Error('network interrupted'))
    mocks.check.mockResolvedValue({
      currentVersion: '0.2.1',
      version: '0.2.2',
      rawJson: {},
      downloadAndInstall: mocks.downloadAndInstall,
    })

    await checkForDesktopUpdate(true)
    await downloadAndInstallDesktopUpdate()

    expect(mocks.invoke).toHaveBeenNthCalledWith(1, 'prepare_for_update')
    expect(mocks.invoke).toHaveBeenNthCalledWith(2, 'cancel_update_preparation')
    expect(updateState.status).toBe('failed')
    expect(updateState.error).toContain('network interrupted')
  })

  it('surfaces current-version failures instead of staying idle', async () => {
    mocks.getVersion.mockRejectedValue(new Error('plugin:app|version not allowed'))

    await checkForDesktopUpdate(true)

    expect(mocks.check).not.toHaveBeenCalled()
    expect(updateState.status).toBe('failed')
    expect(updateState.error).toContain('plugin:app|version not allowed')
  })

  it('restores a recent successful auto-check as up to date after restart', async () => {
    mocks.check.mockResolvedValue(null)

    await checkForDesktopUpdate(false)
    updateState.status = 'idle'
    await checkForDesktopUpdate(false)

    expect(mocks.check).toHaveBeenCalledOnce()
    expect(updateState.currentVersion).toBe('0.2.1')
    expect(updateState.status).toBe('upToDate')
  })
})
