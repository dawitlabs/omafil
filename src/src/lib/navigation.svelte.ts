import { invoke } from '@tauri-apps/api/core'

export type DirectoryEntry = {
  name: string
  path: string
  entryType: 'directory' | 'file'
}

export type PathCrumb = {
  name: string
  path: string
}

export type DirectoryListing = {
  path: string
  crumbs: PathCrumb[]
  entries: DirectoryEntry[]
}

export type DriveInfo = {
  name: string
  mountPoint: string
  path: string
  totalBytes: number
  availableBytes: number
  isRemovable: boolean
  isReadOnly: boolean
}

export type RecentFile = {
  name: string
  path: string
  parentDirectory: string
}

export const knownLocations = ['home', 'desktop', 'documents', 'downloads', 'pictures', 'videos', 'music'] as const

export type KnownLocation = (typeof knownLocations)[number]

export type View = { kind: 'home' } | { kind: 'folder'; path: string } | { kind: 'placeholder'; label: string }

function readableError(error: unknown, fallback: string): string {
  if (typeof error === 'object' && error !== null && 'message' in error && typeof error.message === 'string') {
    return error.message
  }

  return fallback
}

function basename(path: string): string {
  return path.split('/').filter(Boolean).at(-1) ?? path
}

class Navigation {
  #history = $state<View[]>([{ kind: 'home' }])
  #index = $state(0)
  #requestSequence = 0

  listing = $state<DirectoryListing | null>(null)
  isLoading = $state(false)
  error = $state<string | null>(null)

  get view(): View {
    return this.#history[this.#index]
  }

  get canGoBack(): boolean {
    return this.#index > 0
  }

  get canGoForward(): boolean {
    return this.#index < this.#history.length - 1
  }

  get canGoUp(): boolean {
    return this.crumbs.length > 1
  }

  get crumbs(): PathCrumb[] {
    const view = this.view

    return view.kind === 'folder' && this.listing?.path === view.path ? this.listing.crumbs : []
  }

  get label(): string {
    const view = this.view

    if (view.kind === 'home') return 'Home'
    if (view.kind === 'placeholder') return view.label

    return basename(view.path)
  }

  isCurrentPath(path: string): boolean {
    const view = this.view

    return view.kind === 'folder' && view.path === path
  }

  goHome() {
    this.#push({ kind: 'home' })
  }

  openPlaceholder(label: string) {
    this.#push({ kind: 'placeholder', label })
  }

  open(path: string) {
    this.#push({ kind: 'folder', path })
  }

  async openLocation(location: KnownLocation) {
    try {
      this.open(await invoke<string>('resolve_location', { location }))
    } catch (error) {
      this.openPlaceholder(location)
      this.error = readableError(error, 'This location is unavailable on this device.')
    }
  }

  async openEntry(entry: DirectoryEntry) {
    if (entry.entryType === 'directory') {
      this.open(entry.path)
      return
    }

    await this.openExternally(entry.path)
  }

  async openExternally(path: string) {
    this.error = null

    try {
      await invoke('open_path', { path })
    } catch (error) {
      this.error = readableError(error, 'Unable to open this item.')
    }
  }

  back() {
    if (!this.canGoBack) return

    this.#index -= 1
    void this.#load()
  }

  forward() {
    if (!this.canGoForward) return

    this.#index += 1
    void this.#load()
  }

  up() {
    const crumbs = this.crumbs

    if (crumbs.length > 1) this.open(crumbs[crumbs.length - 2].path)
  }

  reload() {
    void this.#load()
  }

  #push(view: View) {
    if (view.kind === 'folder' && this.isCurrentPath(view.path)) return

    this.#history = [...this.#history.slice(0, this.#index + 1), view]
    this.#index = this.#history.length - 1
    void this.#load()
  }

  async #load() {
    const view = this.view
    const requestId = ++this.#requestSequence
    this.error = null

    if (view.kind !== 'folder') {
      this.listing = null
      this.isLoading = false
      return
    }

    this.isLoading = true

    try {
      const listing = await invoke<DirectoryListing>('list_directory', { path: view.path })

      if (requestId === this.#requestSequence) this.listing = listing
    } catch (error) {
      if (requestId === this.#requestSequence) {
        this.listing = null
        this.error = readableError(error, 'Unable to read this folder.')
      }
    } finally {
      if (requestId === this.#requestSequence) this.isLoading = false
    }
  }
}

export const navigation = new Navigation()
