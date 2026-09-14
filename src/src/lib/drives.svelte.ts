import { invoke } from '@tauri-apps/api/core'
import type { ContextMenuItem } from './components/ContextMenu/ContextMenu.svelte'
import { readableError } from './errors'
import type { DriveInfo } from './navigation.svelte'

class DriveStore {
  drives = $state<DriveInfo[]>([])
  isLoading = $state(true)
  error = $state<string | null>(null)
  actionError = $state<string | null>(null)
  formatting = $state<DriveInfo | null>(null)

  async load() {
    this.error = null

    try {
      this.drives = await invoke<DriveInfo[]>('list_drives')
    } catch (caught) {
      this.error = readableError(caught, 'Unable to discover drives.')
    } finally {
      this.isLoading = false
    }
  }

  async #run(command: string, drive: DriveInfo, fallback: string): Promise<string | null> {
    if (!drive.device) return null
    this.actionError = null

    try {
      const result = await invoke<string | null>(command, { device: drive.device })
      await this.load()

      return result
    } catch (caught) {
      this.actionError = readableError(caught, fallback)

      return null
    }
  }

  /** Resolves to the folder to open, or null when the mount failed. */
  async open(drive: DriveInfo): Promise<string | null> {
    if (drive.isMounted) return drive.path

    return this.#run('mount_drive', drive, 'Unable to mount this drive.')
  }

  async unmount(drive: DriveInfo) {
    await this.#run('unmount_drive', drive, 'Unable to unmount this drive. Close any files still open on it.')
  }

  async eject(drive: DriveInfo) {
    await this.#run('eject_drive', drive, 'Unable to eject this drive. Close any files still open on it.')
  }

  /** Resolves to an error message, or null once the drive is formatted. */
  async format(drive: DriveInfo, filesystem: string, label: string): Promise<string | null> {
    if (!drive.device) return 'This drive has no device to format.'

    try {
      await invoke('format_drive', { device: drive.device, filesystem, label })
      await this.load()

      return null
    } catch (caught) {
      return readableError(caught, 'Unable to format this drive.')
    }
  }

  menuItems(drive: DriveInfo, open: (path: string) => void): ContextMenuItem[] {
    const openDrive = async () => {
      const path = await this.open(drive)
      if (path) open(path)
    }

    const formatItem: ContextMenuItem = { kind: 'action', label: 'Format…', onSelect: () => (this.formatting = drive) }

    if (!drive.isRemovable) return [{ kind: 'action', label: 'Open', onSelect: openDrive }, { kind: 'separator' }, formatItem]

    return [
      { kind: 'action', label: drive.isMounted ? 'Open' : 'Mount and open', onSelect: openDrive },
      { kind: 'separator' },
      ...(drive.isMounted ? [{ kind: 'action', label: 'Unmount', onSelect: () => void this.unmount(drive) } as ContextMenuItem] : []),
      { kind: 'action', label: 'Eject', onSelect: () => void this.eject(drive) },
      { kind: 'separator' },
      formatItem,
    ]
  }
}

export const driveStore = new DriveStore()
