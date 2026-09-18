import { invoke } from '@tauri-apps/api/core'
import { readableError } from './errors'

export type AirDropAvailability = {
  available: boolean
  reason: { code: string; message: string }
}

class Sharing {
  paths = $state<string[] | null>(null)
  availability = $state<AirDropAvailability | null>(null)
  error = $state<string | null>(null)
  loading = $state(false)
  private revision = 0
  private timer: ReturnType<typeof setTimeout> | undefined

  open(paths: readonly string[]) {
    if (paths.length === 0) return
    this.paths = [...new Set(paths)]
    void this.check()
  }

  close() {
    this.revision += 1
    clearTimeout(this.timer)
    this.paths = null
    this.availability = null
    this.error = null
    this.loading = false
  }

  async check() {
    const revision = ++this.revision
    clearTimeout(this.timer)
    this.loading = true
    this.error = null
    this.availability = null
    this.timer = setTimeout(() => {
      if (revision !== this.revision) return
      this.revision += 1
      this.loading = false
      this.error = 'Checking AirDrop availability timed out. Try again.'
    }, 5000)
    try {
      const result = await invoke<AirDropAvailability>('airdrop_availability')
      if (revision !== this.revision) return
      this.availability = result
    } catch (error) {
      if (revision !== this.revision) return
      this.error = readableError(error, 'Unable to check AirDrop availability.')
    } finally {
      if (revision === this.revision) {
        clearTimeout(this.timer)
        this.loading = false
      }
    }
  }
}

export const sharing = new Sharing()
