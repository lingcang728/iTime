import { beforeEach, describe, expect, it } from 'vitest'
import { loadPersistedState, savePersistedState, persistedDefaults } from './persistedState'

describe('persisted state validation', () => {
  beforeEach(() => localStorage.clear())

  it('defaults new profiles to the system theme', () => {
    expect(loadPersistedState().theme).toBe('system')
  })

  it('falls back safely when stored data has the wrong shape', () => {
    localStorage.setItem('itime-prototype-state', JSON.stringify({ theme: { malicious: true }, goals: 'bad' }))
    expect(loadPersistedState()).toEqual(persistedDefaults)
  })

  it('migrates only known fields and never restores runtime recording state', () => {
    localStorage.setItem('itime-prototype-state', JSON.stringify({
      theme: 'dark',
      recording: false,
      hideToTray: true,
      goals: { learning: 90, unknown: 999 },
    }))
    const state = loadPersistedState()
    expect(state.theme).toBe('dark')
    expect(state.goals.learning).toBe(90)
    expect(state.goals).not.toHaveProperty('unknown')
    expect(state).not.toHaveProperty('recording')
  })

  it('writes the current schema version', () => {
    savePersistedState(persistedDefaults)
    expect(JSON.parse(localStorage.getItem('itime-prototype-state') ?? '{}').schemaVersion).toBe(2)
  })

  it('drops persisted goal values outside the editable bounds', () => {
    localStorage.setItem('itime-prototype-state', JSON.stringify({
      goals: { learning: 0, development: 1_441, ai: 180.5, continuous: 9 },
    }))
    expect(loadPersistedState().goals).toEqual(persistedDefaults.goals)
  })
})
