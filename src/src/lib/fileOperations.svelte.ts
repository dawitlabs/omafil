import { invoke } from '@tauri-apps/api/core'
import { readableError } from './errors'
import { appState } from './appState.svelte'
import { tabs } from './tabs.svelte'
import type { DirectoryEntry, PathCrumb } from './navigation.svelte'
import { isTerminal, reconcileOperation, remainingCutPaths, type FileOperation } from './operationState'
export type { FileOperation, TransferResult } from './operationState'

export type ClipboardMode = 'copy' | 'cut'
export type FileViewMode = 'details' | 'icons' | 'preview'
export type TransferConflictPolicy = 'fail' | 'skip' | 'replace'
export type TransferResolution = TransferConflictPolicy | 'rename'

export type TransferConflict = {
  sourcePath: string
  destinationPath: string
  name: string
}

type PendingTransfer = {
  paths: string[]
  destinationPath: string
  isMove: boolean
  conflicts: TransferConflict[]
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
  propertiesPath = $state<string | null>(null)
  openWithPath = $state<string | null>(null)
  bulkRenamePaths = $state<string[] | null>(null)
  viewMode = $state<FileViewMode>('details')
  pendingTransfer = $state<PendingTransfer | null>(null)
  pendingPermanentDelete = $state<string[] | null>(null)
  archiveDraft = $state('Archive.zip')
  pendingArchivePaths = $state<string[] | null>(null)
  archiveDestination = $state('')
  externalDropPaths = $state<string[] | null>(null)
  error = $state<string | null>(null)
  isBusy = $state(false)
  operations = $state<FileOperation[]>([])

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

  #setClipboard(paths: string[], mode: ClipboardMode) {
    if (paths.length === 0) return

    this.clipboardPaths = paths
    this.clipboardMode = mode
    void invoke('write_file_clipboard', { paths, isCut: mode === 'cut' }).catch(() => {})
  }

  /** Picks up files copied in another file manager, so Ctrl+V works across apps. */
  async syncFromDesktopClipboard() {
    const clipboard = await invoke<[string[], boolean] | null>('read_file_clipboard').catch(() => null)
    if (!clipboard) return

    const [paths, isCut] = clipboard
    if (paths.join('\n') === this.clipboardPaths.join('\n')) return

    this.clipboardPaths = paths
    this.clipboardMode = isCut ? 'cut' : 'copy'
  }

  copySelection() {
    this.#setClipboard(this.selectedPaths, 'copy')
  }

  copyPaths(paths: string[]) {
    this.#setClipboard(paths, 'copy')
  }

  cutPaths(paths: string[]) {
    this.#setClipboard(paths, 'cut')
  }

  cutSelection() {
    this.#setClipboard(this.selectedPaths, 'cut')
  }

  showBulkRename(paths: string[] = this.selectedPaths) {
    if (paths.length > 1) this.bulkRenamePaths = paths
  }

  hideBulkRename() {
    this.bulkRenamePaths = null
  }

  async applyBulkRename(items: Array<{ path: string; to: string }>) {
    await this.#run(async () => {
      let failed = 0

      // Sequential on purpose: a plan may chain names (a→b while b→c), so
      // order decides whether the second rename finds its target free.
      for (const { path, to } of items) {
        try {
          appState.relocate(path, await invoke<string>('rename_path', { path, name: to }))
        } catch {
          failed += 1
        }
      }

      this.bulkRenamePaths = null
      this.clearSelection()
      if (failed > 0) throw new Error(`${failed} of ${items.length} items could not be renamed. The rest were.`)
    }, 'Unable to rename those items.')
  }

  startRenaming(path: string = this.selectedPaths[0]) {
    if (!path) return
    if (this.selectedPaths.length > 1 && this.isSelected(path)) {
      this.showBulkRename()
      return
    }

    this.renameDraft = this.entries.find((entry) => entry.path === path)?.name ?? ''
    this.renamingPath = path
  }

  cancelRenaming() {
    this.renamingPath = null
  }

  showOpenWith(path: string = this.selectedPaths[0]) {
    if (path) this.openWithPath = path
  }

  hideOpenWith() {
    this.openWithPath = null
  }

  showProperties(path: string = this.selectedPaths[0]) {
    if (path) this.propertiesPath = path
  }

  hideProperties() {
    this.propertiesPath = null
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
    await this.createFolderIn(this.directoryPath)
  }

  /** Resolves to the new folder, or null when it could not be created. */
  async createFolderIn(parentPath: string): Promise<string | null> {
    if (!parentPath) return null

    const siblings =
      parentPath === this.directoryPath
        ? this.entries.map((entry) => entry.name)
        : await invoke<PathCrumb[]>('list_subdirectories', { path: parentPath, showHidden: true })
            .then((folders) => folders.map((folder) => folder.name))
            .catch(() => [])
    const name = untakenFolderName(siblings)
    let created: string | null = null

    await this.#run(async () => {
      created = await invoke<string>('new_directory', { parentPath, name })

      if (parentPath === this.directoryPath) {
        this.selectOnly(created)
        this.renameDraft = name
        this.renamingPath = created
      }
    }, 'Unable to create that folder.')

    return created
  }

  async rename(path: string, name: string = this.renameDraft) {
    this.renamingPath = null

    const entry = this.entries.find((candidate) => candidate.path === path)

    if (!entry || name.trim() === entry.name) return

    const renamed = await this.renamePath(path, name)
    if (renamed) this.selectOnly(renamed)
  }

  /** Renames anything by path, whether or not it is in the folder on screen. */
  async renamePath(path: string, name: string): Promise<string | null> {
    const current = path.split('/').filter(Boolean).at(-1) ?? path

    if (!name.trim() || name.trim() === current) return null

    let renamed: string | null = null

    await this.#run(async () => {
      renamed = await invoke<string>('rename_path', { path, name })
      appState.relocate(path, renamed)
    }, 'Unable to rename that item.')

    return renamed
  }

  async deleteSelection() {
    await this.deletePaths(this.selectedPaths)
  }

  async deletePaths(paths: string[]) {
    if (paths.length === 0) return

    const deleted = await this.#run(() => invoke('trash_paths', { paths }), 'Unable to move those items to the trash.')

    if (deleted) {
      appState.forget(paths)
      this.clearSelection()
    }
  }

  requestPermanentDelete(paths: string[] = this.selectedPaths) {
    if (paths.length > 0) this.pendingPermanentDelete = [...paths]
  }

  cancelPermanentDelete() {
    this.pendingPermanentDelete = null
  }

  async confirmPermanentDelete() {
    const paths = this.pendingPermanentDelete
    if (!paths) return

    this.pendingPermanentDelete = null
    const deleted = await this.#run(() => invoke('permanently_delete_paths', { paths }), 'Unable to permanently delete those items.')
    if (deleted) {
      appState.forget(paths)
      this.clearSelection()
    }
  }

  async #transfer(paths: string[], destinationPath: string, isMove: boolean, conflictPolicy: TransferResolution): Promise<boolean> {
    try {
      const queued = await invoke<{ id: string }>('queue_transfer', { paths, destinationPath, isMove, conflictPolicy })
      this.operations = reconcileOperation(this.operations, { id: queued.id, kind: isMove ? 'move' : 'copy', state: 'queued', completedItems: 0, totalItems: paths.length, completedBytes: 0, totalBytes: null, currentName: null })
      return true
    } catch (error) {
      this.error = readableError(error, isMove ? 'Unable to queue those items to move.' : 'Unable to queue those items to copy.')
      return false
    }
  }

  receiveOperationUpdate(update: FileOperation) {
    const previous = this.operations.find((operation) => operation.id === update.id)
    if (previous && isTerminal(previous)) return
    this.operations = reconcileOperation(this.operations, update)

    if (isTerminal(update)) {
      if (update.kind === 'move' && update.results) {
        for (const result of update.results) {
          if (!result.skipped) appState.relocate(result.sourcePath, result.destinationPath)
        }
        if (this.clipboardMode === 'cut') {
          this.clipboardPaths = remainingCutPaths(this.clipboardPaths, update.results)
          if (this.clipboardPaths.length === 0) this.clipboardMode = null
        }
      }
      tabs.active.reload()
    }
  }

  async cancelOperation(id: string) {
    const operation = this.operations.find((candidate) => candidate.id === id)
    if (!operation || isTerminal(operation) || operation.cancellationRequested) return
    this.operations = this.operations.map((candidate) => candidate.id === id
      ? { ...candidate, cancellationRequested: true, cancellationError: null } : candidate)
    try {
      const accepted = await invoke<boolean>('cancel_operation', { id })
      if (!accepted) {
        this.operations = this.operations.map((candidate) => candidate.id === id
          ? { ...candidate, cancellationRequested: false } : candidate)
      }
    } catch (error) {
      this.operations = this.operations.map((candidate) => candidate.id === id
        ? { ...candidate, cancellationRequested: false, cancellationError: readableError(error, 'Unable to cancel this operation. Try again.') } : candidate)
    }
  }

  dismissOperation(id: string) {
    this.operations = this.operations.filter((operation) => operation.id !== id)
  }

  async beginTransfer(paths: string[], destinationPath: string, isMove: boolean) {
    if (paths.length === 0 || !destinationPath) return

    this.error = null

    try {
      const conflicts = await invoke<TransferConflict[]>('transfer_conflicts', { paths, destinationPath })

      if (conflicts.length > 0) {
        this.pendingTransfer = { paths, destinationPath, isMove, conflicts }
        return
      }

      await this.#transfer(paths, destinationPath, isMove, 'fail')
    } catch (error) {
      this.error = readableError(error, 'Unable to prepare that transfer.')
    }
  }

  async resolvePendingTransfer(policy: Exclude<TransferResolution, 'fail'>) {
    const pending = this.pendingTransfer

    if (!pending) return

    this.pendingTransfer = null
    await this.#transfer(pending.paths, pending.destinationPath, pending.isMove, policy)
  }

  cancelPendingTransfer() {
    this.pendingTransfer = null
  }

  async paste() {
    await this.syncFromDesktopClipboard()
    if (!this.canPaste) return

    await this.beginTransfer(this.clipboardPaths, this.directoryPath, this.clipboardMode === 'cut')
  }

  async pasteTo(destinationPath: string) {
    if (this.clipboardMode === null || this.clipboardPaths.length === 0) return
    await this.beginTransfer(this.clipboardPaths, destinationPath, this.clipboardMode === 'cut')
  }

  async compressSelection() {
    this.compressPaths(this.selectedPaths, this.directoryPath)
  }

  /** ZIPs anything by path, writing the archive into destinationPath. */
  compressPaths(paths: string[], destinationPath: string) {
    if (paths.length === 0 || !destinationPath) return

    const only = paths.length === 1 ? (paths[0].split('/').filter(Boolean).at(-1) ?? 'Archive') : null
    this.archiveDraft = `${only ?? 'Archive'}.zip`
    this.archiveDestination = destinationPath
    this.pendingArchivePaths = [...paths]
  }

  cancelArchive() { this.pendingArchivePaths = null }

  async createArchive() {
    const paths = this.pendingArchivePaths
    const name = this.archiveDraft.trim()
    const destinationPath = this.archiveDestination || this.directoryPath
    if (!paths || !name || !destinationPath) return
    this.pendingArchivePaths = null
    try {
      const queued = await invoke<{ id: string }>('queue_create_zip', { paths, destinationPath, name })
      this.operations = reconcileOperation(this.operations, { id: queued.id, kind: 'compress', state: 'queued', completedItems: 0, totalItems: 1, completedBytes: 0, totalBytes: null, currentName: name })
    } catch (error) {
      this.error = readableError(error, 'Unable to queue the ZIP archive.')
    }
  }

  async extractSelection() {
    const path = this.selectedPaths[0]
    if (!path || !this.directoryPath || !path.toLowerCase().endsWith('.zip')) return
    try {
      const queued = await invoke<{ id: string }>('queue_extract_zip', { path, destinationPath: this.directoryPath })
      this.operations = reconcileOperation(this.operations, { id: queued.id, kind: 'extract', state: 'queued', completedItems: 0, totalItems: 1, completedBytes: 0, totalBytes: null, currentName: path.split('/').at(-1) ?? path })
    } catch (error) {
      this.error = readableError(error, 'Unable to queue this ZIP extraction.')
    }
  }

  async moveSelectionTo(destinationPath: string, copy = false) {
    await this.movePathsTo(this.selectedPaths, destinationPath, copy)
  }

  async movePathsTo(paths: string[], destinationPath: string, copy = false) {
    await this.beginTransfer(paths, destinationPath, !copy)
  }

  async importDroppedPaths(paths: string[]) {
    await this.beginTransfer(paths, this.directoryPath, false)
  }
}

export const fileOperations = new FileOperations()
