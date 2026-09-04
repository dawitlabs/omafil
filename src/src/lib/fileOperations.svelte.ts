import { invoke } from '@tauri-apps/api/core'
import { tabs } from './tabs.svelte'
import type { DirectoryEntry } from './navigation.svelte'

export type ClipboardMode = 'copy' | 'cut'

function readableError(error: unknown, fallback: string): string {
  if (typeof error === 'object' && error !== null && 'message' in error && typeof error.message === 'string') {
    return error.message
  }

  return fallback
}

function untakenFolderName(existingNames: string[]): string {
  const base = 'New folder'

  if (!existingNames.includes(base)) return base

  let suffix = 2
  while (existingNames.includes(`${base} (${suffix})`)) suffix += 1

  return `${base} (${suffix})`
}

class FileOperations {
  #selection = $state<string[]>([])
  #selectionScope = $state('')
  #anchorPath = $state<string | null>(null)

  clipboardPaths = $state<string[]>([])
  clipboardMode = $state<ClipboardMode | null>(null)
  renamingPath = $state<string | null>(null)
  renameDraft = $state('')
  error = $state<string | null>(null)
  isBusy = $state(false)

  get entries(): DirectoryEntry[] {
    return tabs.active.entries
  }

  get directoryPath(): string {
    return tabs.active.directoryPath
  }

  get selectedPaths(): string[] {
    return this.#selectionScope === this.directoryPath ? this.#selection : []
  }

  get canPaste(): boolean {
    return this.clipboardMode !== null && this.clipboardPaths.length > 0 && this.directoryPath !== ''
  }

  isSelected(path: string): boolean {
    return this.selectedPaths.includes(path)
  }

  #setSelection(paths: string[]) {
    this.#selection = paths
    this.#selectionScope = this.directoryPath
  }

  clearSelection() {
    this.#setSelection([])
    this.#anchorPath = null
  }

  selectAll() {
    this.#setSelection(this.entries.map((entry) => entry.path))
  }

  selectOnly(path: string) {
    this.#setSelection([path])
    this.#anchorPath = path
  }

  toggle(path: string) {
    const selected = this.selectedPaths

    this.#setSelection(selected.includes(path) ? selected.filter((candidate) => candidate !== path) : [...selected, path])
    this.#anchorPath = path
  }

  extendTo(path: string) {
    const paths = this.entries.map((entry) => entry.path)
    const anchorIndex = paths.indexOf(this.#anchorPath ?? path)
    const targetIndex = paths.indexOf(path)

    if (anchorIndex === -1 || targetIndex === -1) {
      this.selectOnly(path)
      return
    }

    const [start, end] = anchorIndex <= targetIndex ? [anchorIndex, targetIndex] : [targetIndex, anchorIndex]
    this.#setSelection(paths.slice(start, end + 1))
  }

  selectWithin(paths: string[]) {
    this.#setSelection(paths)
  }

  copySelection() {
    if (this.selectedPaths.length === 0) return

    this.clipboardPaths = this.selectedPaths
    this.clipboardMode = 'copy'
  }

  cutSelection() {
    if (this.selectedPaths.length === 0) return

    this.clipboardPaths = this.selectedPaths
    this.clipboardMode = 'cut'
  }

  startRenaming(path: string = this.selectedPaths[0]) {
    if (!path) return

    this.renameDraft = this.entries.find((entry) => entry.path === path)?.name ?? ''
    this.renamingPath = path
  }

  cancelRenaming() {
    this.renamingPath = null
  }

  async #run(operation: () => Promise<unknown>, fallback: string): Promise<boolean> {
    this.isBusy = true
    this.error = null

    try {
      await operation()
      tabs.active.reload()
      return true
    } catch (error) {
      this.error = readableError(error, fallback)
      return false
    } finally {
      this.isBusy = false
    }
  }

  async createFolder() {
    const parentPath = this.directoryPath

    if (!parentPath) return

    const name = untakenFolderName(this.entries.map((entry) => entry.name))

    await this.#run(async () => {
      const created = await invoke<string>('new_directory', { parentPath, name })
      this.selectOnly(created)
      this.renameDraft = name
      this.renamingPath = created
    }, 'Unable to create that folder.')
  }

  async rename(path: string, name: string = this.renameDraft) {
    this.renamingPath = null

    const entry = this.entries.find((candidate) => candidate.path === path)

    if (!entry || name.trim() === entry.name) return

    await this.#run(async () => {
      const renamed = await invoke<string>('rename_path', { path, name })
      this.selectOnly(renamed)
    }, 'Unable to rename that item.')
  }

  async deleteSelection() {
    const paths = this.selectedPaths

    if (paths.length === 0) return

    const deleted = await this.#run(() => invoke('trash_paths', { paths }), 'Unable to move those items to the trash.')

    if (deleted) this.clearSelection()
  }

  async paste() {
    if (!this.canPaste) return

    const paths = this.clipboardPaths
    const isMove = this.clipboardMode === 'cut'
    const pasted = await this.#run(
      () => invoke('transfer_paths', { paths, destinationPath: this.directoryPath, isMove }),
      isMove ? 'Unable to move those items here.' : 'Unable to copy those items here.',
    )

    if (pasted && isMove) {
      this.clipboardPaths = []
      this.clipboardMode = null
    }
  }
}

export const fileOperations = new FileOperations()
