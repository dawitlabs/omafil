import { invoke } from '@tauri-apps/api/core'
import { readableError } from './errors'

export type ServiceLocation = { kind: 'local'; path: string } | { kind: 'remote'; uri: string }
export type RemoteEntry = { uri: string; name: string; isDirectory: boolean; isRegular: boolean; size: number; modified: number }
export type RemoteListing = { uri: string; parent: string | null; entries: RemoteEntry[]; truncated: boolean }
export type ServiceMount = { name: string; uri: string; mounted: boolean; canUnmount: boolean }
export type CopyReport = { copied: number; failures: Array<{ name: string; message: string }> }

/** One cancellable request per surface. Late results cannot change a closed view. */
export class ServiceTask {
  busy = $state(false)
  error = $state<string | null>(null)
  progress = $state<{ completed: number; total: number } | null>(null)
  id = $state<string | null>(null)

  async run<T>(operation: string, payload: Record<string, unknown> = {}): Promise<T | null> {
    this.dispose()
    const id = crypto.randomUUID()
    this.id = id
    this.busy = true
    this.error = null
    this.progress = null
    try {
      const value = await invoke<T>('linux_service', { id, operation, payload })
      return this.id === id ? value : null
    } catch (error) {
      if (this.id === id) this.error = readableError(error, 'The Linux service is unavailable. Check the required packages and retry.')
      return null
    } finally {
      if (this.id === id) { this.busy = false; this.id = null }
    }
  }

  cancel() {
    if (this.id) void invoke('cancel_linux_service', { id: this.id }).catch(() => {})
  }

  dispose() {
    this.cancel()
    this.id = null
    this.busy = false
  }
}

export function displayAddress(uri: string): string {
  return uri.replace(/(\/\/)[^/]*@/, '$1')
}
