import { invoke } from '@tauri-apps/api/core'
import { readableError } from './errors'
import { appState } from './appState.svelte'

export type DirectoryEntry = {
  name: string
  path: string
  entryType: 'directory' | 'file'
  size: number
  modified: number | null
}

export type EntrySort = 'name' | 'modified' | 'type' | 'size'

export type PathCrumb = {
  name: string
  path: string
}

export type DirectoryListing = {
  path: string
  crumbs: PathCrumb[]
  entries: DirectoryEntry[]
  total: number
  hasMore: boolean
}

const DIRECTORY_PAGE_SIZE = 300

const descendingByDefault: EntrySort[] = ['size', 'modified']

export type DriveInfo = {
  name: string
  mountPoint: string
  path: string
  totalBytes: number
  availableBytes: number
  isRemovable: boolean
  isReadOnly: boolean
  isMounted: boolean
  device: string | null
}

export type RecentFile = {
  name: string
  path: string
  parentDirectory: string
}

export const knownLocations = ['home', 'desktop', 'documents', 'downloads', 'pictures', 'videos', 'music'] as const

export type KnownLocation = (typeof knownLocations)[number]

export type SearchResults = {
  entries: DirectoryEntry[]
  truncated: boolean
}

export type View =
  | { kind: 'home' }
  | { kind: 'settings' }
  | { kind: 'recycle' }
  | { kind: 'folder'; path: string }
  | { kind: 'search'; path: string; query: string }
  | { kind: 'tag'; id: string; label: string }
  | { kind: 'placeholder'; label: string }

function basename(path: string): string {
  return path.split('/').filter(Boolean).at(-1) ?? path
}

export class Navigation {
  /** Second pane shown beside this one while the tab is split. */
  split = $state<Navigation | null>(null)
  #history = $state<View[]>([{ kind: 'home' }])
  #index = $state(0)
  #requestSequence = 0

  listing = $state<DirectoryListing | null>(null)
  results = $state<SearchResults | null>(null)
  missingTagged = $state(0)
  isLoading = $state(false)
  isLoadingMore = $state(false)
  error = $state<string | null>(null)
  #sort = $state<EntrySort | null>(null)
  #descending = $state<boolean | null>(null)

  get sort(): EntrySort {
    return this.#sort ?? appState.settings.defaultSort
  }

  get descending(): boolean {
    return this.#descending ?? appState.settings.defaultDescending
  }

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

  get entries(): DirectoryEntry[] {
    if (this.view.kind === 'folder') return this.listing?.entries ?? []

    return this.results?.entries ?? []
  }

  get total(): number {
    if (this.view.kind === 'folder') return this.listing?.total ?? 0

    return this.results?.entries.length ?? 0
  }

  get hiddenCount(): number {
    if (this.view.kind !== 'folder') return 0

    return (this.listing?.total ?? 0) - (this.listing?.entries.length ?? 0)
  }

  get isSearchTruncated(): boolean {
    return this.view.kind === 'search' && (this.results?.truncated ?? false)
  }

  get hasMoreEntries(): boolean {
    return this.view.kind === 'folder' && (this.listing?.hasMore ?? false)
  }

  get directoryPath(): string {
    return this.view.kind === 'folder' ? (this.listing?.path ?? '') : ''
  }

  get crumbs(): PathCrumb[] {
    const view = this.view

    return view.kind === 'folder' && this.listing?.path === view.path ? this.listing.crumbs : []
  }

  get label(): string {
    const view = this.view

    if (view.kind === 'home') return 'Home'
    if (view.kind === 'settings') return 'Settings'
    if (view.kind === 'recycle') return 'Recycle Bin'
    if (view.kind === 'placeholder') return view.label
    if (view.kind === 'tag') return view.label
    if (view.kind === 'search') return `Search: ${view.query}`

    return basename(view.path)
  }

  isCurrentPath(path: string): boolean {
    const view = this.view

    return view.kind === 'folder' && view.path === path
  }

  goHome() {
    this.#push({ kind: 'home' })
  }

  openSettings() {
    this.#push({ kind: 'settings' })
  }

  openRecycleBin() {
    this.#push({ kind: 'recycle' })
  }

  openPlaceholder(label: string) {
    this.#push({ kind: 'placeholder', label })
  }

  open(path: string) {
    this.#push({ kind: 'folder', path })
  }

  get searchScope(): string {
    const view = this.view

    if (view.kind === 'search') return basename(view.path)

    return this.directoryPath ? basename(this.directoryPath) : 'your files'
  }

  async search(query: string) {
    const view = this.view
    const scoped = view.kind === 'search' ? view.path : this.directoryPath

    // Home and the tag, settings and placeholder views have no folder of their
    // own, so a search from there covers everything the user can reach.
    const path = scoped || (await invoke<string>('resolve_location', { location: 'home' }).catch(() => ''))

    if (!path) return

    this.#push({ kind: 'search', path, query })
  }

  openTag(id: string, label: string) {
    this.#push({ kind: 'tag', id, label })
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
    await this.#launch('open_path', path, 'Unable to open this item.')
  }

  async openTerminal(path = this.directoryPath) {
    if (path) await this.#launch('open_terminal', path, 'Unable to open a terminal here.')
  }

  async openInEditor(path: string) {
    await this.#launch('open_in_editor', path, 'Unable to open this file in your editor.')
  }

  async #launch(command: string, path: string, fallback: string) {
    this.error = null

    try {
      await invoke(command, { path })
    } catch (error) {
      this.error = readableError(error, fallback)
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

  loadMore() {
    const listing = this.listing
    if (this.view.kind !== 'folder' || !listing?.hasMore || this.isLoadingMore) return
    void this.#loadMore(listing)
  }

  sortBy(sort: EntrySort) {
    this.#descending = this.sort === sort ? !this.descending : descendingByDefault.includes(sort)
    this.#sort = sort
    void this.#load()
  }

  #push(view: View) {
    if (view.kind === 'folder' && this.isCurrentPath(view.path)) return

    const current = this.view

    if (view.kind === 'search' && current.kind === 'search' && current.path === view.path) {
      this.#history = [...this.#history.slice(0, this.#index), view]
      void this.#load()
      return
    }

    this.#history = [...this.#history.slice(0, this.#index + 1), view]
    this.#index = this.#history.length - 1
    void this.#load()
  }

  async #load() {
    const view = this.view
    const requestId = ++this.#requestSequence
    this.error = null
    this.missingTagged = 0

    if (view.kind === 'home' || view.kind === 'placeholder' || view.kind === 'settings' || view.kind === 'recycle') {
      this.listing = null
      this.results = null
      this.isLoading = false
      return
    }

    this.isLoading = true

    try {
      if (view.kind === 'folder') {
        const listing = await invoke<DirectoryListing>('list_directory', {
          path: view.path,
          sort: this.sort,
          descending: this.descending,
          showHidden: appState.settings.showHidden,
          offset: 0,
          limit: DIRECTORY_PAGE_SIZE,
        })

        if (requestId === this.#requestSequence) {
          this.listing = listing
          this.results = null
        }
      } else if (view.kind === 'search') {
        const results = await invoke<SearchResults>('search_files', {
          path: view.path,
          query: view.query,
          showHidden: appState.settings.showHidden,
        })

        if (requestId === this.#requestSequence) {
          this.listing = null
          this.results = results
        }
      } else {
        const described = await this.#describeTagged(view.id)

        if (requestId === this.#requestSequence) {
          this.listing = null
          this.results = { entries: described.entries, truncated: false }
          this.missingTagged = described.missing
        }
      }
    } catch (error) {
      if (requestId === this.#requestSequence) {
        this.listing = null
        this.results = null
        this.error = readableError(error, 'Unable to read this location.')
      }
    } finally {
      if (requestId === this.#requestSequence) this.isLoading = false
    }
  }

  async #describeTagged(id: string): Promise<{ entries: DirectoryEntry[]; missing: number }> {
    const described = await Promise.all(
      appState.pathsWithTag(id).map((path) => invoke<DirectoryEntry | null>('describe_path', { path }).catch(() => null)),
    )
    const entries = described.filter((entry): entry is DirectoryEntry => entry !== null)

    // Not pruned: a tagged item on an unmounted drive is unreachable, not gone.
    return { entries, missing: described.length - entries.length }
  }

  async #loadMore(current: DirectoryListing) {
    const view = this.view
    if (view.kind !== 'folder') return

    const requestId = this.#requestSequence
    this.isLoadingMore = true

    try {
      const next = await invoke<DirectoryListing>('list_directory', {
        path: view.path,
        sort: this.sort,
        descending: this.descending,
        showHidden: appState.settings.showHidden,
        offset: current.entries.length,
        limit: DIRECTORY_PAGE_SIZE,
      })

      if (requestId === this.#requestSequence && this.listing?.path === current.path) {
        this.listing = { ...next, entries: [...current.entries, ...next.entries] }
      }
    } catch (error) {
      if (requestId === this.#requestSequence) this.error = readableError(error, 'Unable to load more items.')
    } finally {
      if (requestId === this.#requestSequence) this.isLoadingMore = false
    }
  }
}
