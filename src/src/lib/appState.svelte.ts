import { invoke } from '@tauri-apps/api/core'

export type PinnedLocation = {
  path: string
  label: string
}

export type Tag = {
  id: string
  label: string
  color: string
}

export type Settings = {
  showHidden: boolean
}

type StoredState = {
  pins: PinnedLocation[]
  tags: Tag[]
  tagged: Record<string, string[]>
  settings: Settings
}

const SAVE_DELAY_MS = 400

export const tagColors = ['#f5342e', '#ffc72c', '#0a84e8', '#28a745', '#a855f7'] as const

const seedTags: Tag[] = [
  { id: 'design', label: 'Design', color: '#f5342e' },
  { id: 'dev', label: 'Dev', color: '#ffc72c' },
  { id: 'school', label: 'School', color: '#0a84e8' },
]

function basename(path: string): string {
  return path.split('/').filter(Boolean).at(-1) ?? path
}

class AppState {
  pins = $state<PinnedLocation[]>([])
  tags = $state<Tag[]>([])
  tagged = $state<Record<string, string[]>>({})
  settings = $state<Settings>({ showHidden: false })
  isLoaded = $state(false)
  error = $state<string | null>(null)

  #saveTimer: ReturnType<typeof setTimeout> | null = null

  async load() {
    const stored = await invoke<StoredState>('load_state')
    const isFirstRun = stored.tags.length === 0 && stored.pins.length === 0 && Object.keys(stored.tagged).length === 0

    this.pins = stored.pins
    this.tags = isFirstRun ? seedTags : stored.tags
    this.tagged = stored.tagged
    this.settings = stored.settings
    this.isLoaded = true

    if (isFirstRun) await this.#seedPins()
  }

  async #seedPins() {
    try {
      const desktop = await invoke<string>('resolve_location', { location: 'desktop' })

      this.pins = [{ path: desktop, label: basename(desktop) }]
      this.#persist()
    } catch {
      this.#persist()
    }
  }

  #persist() {
    if (this.#saveTimer) clearTimeout(this.#saveTimer)

    this.#saveTimer = setTimeout(async () => {
      try {
        await invoke('save_state', {
          state: {
            pins: this.pins,
            tags: this.tags,
            tagged: this.tagged,
            settings: this.settings,
          },
        })
        this.error = null
      } catch {
        this.error = 'Unable to save your pins, tags and settings.'
      }
    }, SAVE_DELAY_MS)
  }

  isPinned(path: string): boolean {
    return this.pins.some((pin) => pin.path === path)
  }

  togglePin(path: string) {
    this.pins = this.isPinned(path) ? this.pins.filter((pin) => pin.path !== path) : [...this.pins, { path, label: basename(path) }]
    this.#persist()
  }

  setShowHidden(showHidden: boolean) {
    this.settings = { ...this.settings, showHidden }
    this.#persist()
  }

  createTag(label: string): Tag {
    const color = tagColors[this.tags.length % tagColors.length]
    const tag = { id: `${Date.now().toString(36)}`, label, color }

    this.tags = [...this.tags, tag]
    this.#persist()

    return tag
  }

  removeTag(id: string) {
    this.tags = this.tags.filter((tag) => tag.id !== id)
    this.tagged = Object.fromEntries(
      Object.entries(this.tagged)
        .map(([path, ids]) => [path, ids.filter((tagId) => tagId !== id)] as const)
        .filter(([, ids]) => ids.length > 0),
    )
    this.#persist()
  }

  tagsFor(path: string): Tag[] {
    const ids = this.tagged[path] ?? []

    return this.tags.filter((tag) => ids.includes(tag.id))
  }

  pathsWithTag(id: string): string[] {
    return Object.entries(this.tagged)
      .filter(([, ids]) => ids.includes(id))
      .map(([path]) => path)
  }

  toggleTag(path: string, id: string) {
    const ids = this.tagged[path] ?? []
    const next = ids.includes(id) ? ids.filter((tagId) => tagId !== id) : [...ids, id]
    const { [path]: _removed, ...rest } = this.tagged

    this.tagged = next.length > 0 ? { ...rest, [path]: next } : rest
    this.#persist()
  }
}

export const appState = new AppState()
