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

export type Theme = 'system' | 'light' | 'dark'

export type SortKey = 'name' | 'modified' | 'type' | 'size'

export type Settings = {
  showHidden: boolean
  theme: Theme
  defaultSort: SortKey
  defaultDescending: boolean
  vimKeys: boolean
}

const defaultSettings: Settings = {
  showHidden: false,
  theme: 'system',
  defaultSort: 'name',
  defaultDescending: false,
  vimKeys: false,
}

type StoredState = {
  pins: PinnedLocation[]
  tags: Tag[]
  tagged: Record<string, string[]>
  settings: Settings
}

const SAVE_DELAY_MS = 400

type OmarchyColors = Record<string, string>

function omarchyVariable(key: string): string {
  return `--om-${key.replaceAll('_', '-')}`
}

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
  settings = $state<Settings>({ ...defaultSettings })
  isLoaded = $state(false)
  error = $state<string | null>(null)
  omarchyColors = $state<OmarchyColors | null>(null)

  #appliedOmarchyVariables: string[] = []
  #saveTimer: ReturnType<typeof setTimeout> | null = null
  #darkMedia = window.matchMedia('(prefers-color-scheme: dark)')

  constructor() {
    this.#darkMedia.addEventListener('change', () => this.applyTheme())
    this.applyTheme()
  }

  get isOmarchy(): boolean {
    return this.omarchyColors !== null
  }

  get resolvedTheme(): 'light' | 'dark' {
    if (this.settings.theme !== 'system') return this.settings.theme
    if (this.omarchyColors) return this.omarchyColors.mode === 'light' ? 'light' : 'dark'

    return this.#darkMedia.matches ? 'dark' : 'light'
  }

  applyTheme() {
    const root = document.documentElement
    const colors = this.settings.theme === 'system' ? this.omarchyColors : null

    root.dataset.theme = colors ? 'omarchy' : this.resolvedTheme
    root.dataset.mode = this.resolvedTheme

    for (const variable of this.#appliedOmarchyVariables) root.style.removeProperty(variable)
    this.#appliedOmarchyVariables = []
    if (!colors) return

    for (const [key, value] of Object.entries(colors)) {
      const variable = omarchyVariable(key)
      root.style.setProperty(variable, value)
      this.#appliedOmarchyVariables.push(variable)
    }
  }

  async refreshOmarchyTheme() {
    this.omarchyColors = await invoke<OmarchyColors | null>('read_omarchy_theme')
    this.applyTheme()
  }

  async load() {
    const [stored, omarchyColors] = await Promise.all([
      invoke<StoredState>('load_state'),
      invoke<OmarchyColors | null>('read_omarchy_theme'),
    ])

    this.omarchyColors = omarchyColors
    const isFirstRun = stored.tags.length === 0 && stored.pins.length === 0 && Object.keys(stored.tagged).length === 0

    this.pins = stored.pins
    this.tags = isFirstRun ? seedTags : stored.tags
    this.tagged = stored.tagged
    this.settings = { ...defaultSettings, ...stored.settings }
    this.isLoaded = true
    this.applyTheme()

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

  /// A stored path follows the item it names when the app moves or renames it,
  /// including everything underneath a renamed folder.
  relocate(from: string, to: string) {
    const rewrite = (path: string) => {
      if (path === from) return to

      return path.startsWith(`${from}/`) ? `${to}${path.slice(from.length)}` : path
    }

    this.pins = this.pins.map((pin) => {
      const path = rewrite(pin.path)

      return path === pin.path ? pin : { path, label: pin.path === from ? basename(to) : pin.label }
    })

    this.tagged = Object.fromEntries(Object.entries(this.tagged).map(([path, ids]) => [rewrite(path), ids]))
    this.#persist()
  }

  forget(paths: string[]) {
    const isGone = (candidate: string) => paths.some((path) => candidate === path || candidate.startsWith(`${path}/`))

    this.pins = this.pins.filter((pin) => !isGone(pin.path))
    this.tagged = Object.fromEntries(Object.entries(this.tagged).filter(([path]) => !isGone(path)))
    this.#persist()
  }

  isPinned(path: string): boolean {
    return this.pins.some((pin) => pin.path === path)
  }

  unpin(path: string) {
    this.pins = this.pins.filter((pin) => pin.path !== path)
    this.#persist()
  }

  togglePin(path: string) {
    this.pins = this.isPinned(path) ? this.pins.filter((pin) => pin.path !== path) : [...this.pins, { path, label: basename(path) }]
    this.#persist()
  }

  update(changes: Partial<Settings>) {
    this.settings = { ...this.settings, ...changes }
    this.applyTheme()
    this.#persist()
  }

  setShowHidden(showHidden: boolean) {
    this.update({ showHidden })
  }

  createTag(label: string): Tag {
    const color = tagColors[this.tags.length % tagColors.length]
    const tag = { id: crypto.randomUUID(), label, color }

    this.tags = [...this.tags, tag]
    this.#persist()

    return tag
  }

  updateTag(id: string, changes: Partial<Pick<Tag, 'label' | 'color'>>) {
    this.tags = this.tags.map((tag) => (tag.id === id ? { ...tag, ...changes } : tag))
    this.#persist()
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
