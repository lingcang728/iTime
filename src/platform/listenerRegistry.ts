export type ListenerCleanup = () => void
export type ListenerRegistration = () => Promise<ListenerCleanup>

export async function registerListenersIndependently(
  registrations: ListenerRegistration[],
  onError: (error: unknown) => void,
): Promise<ListenerCleanup[]> {
  const cleanups = await Promise.all(registrations.map(async (register) => {
    try {
      return await register()
    } catch (error) {
      onError(error)
      return null
    }
  }))
  return cleanups.filter((cleanup): cleanup is ListenerCleanup => cleanup !== null)
}
