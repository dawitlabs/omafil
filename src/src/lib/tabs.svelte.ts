import { tick } from 'svelte'
import { Navigation } from './navigation.svelte'

const MAX_TABS = 12

class Tabs {
  #tabs = $state<Navigation[]>([new Navigation()])
  activeIndex = $state(0)
  error = $state<string | null>(null)

  get all(): Navigation[] {
    return this.#tabs
  }

  focusedSide = $state<'left' | 'right'>('left')

  /** The tab itself: the left pane, which owns any split. */
  get tab(): Navigation {
    return this.#tabs[this.activeIndex]
  }

  /** The pane keyboard and toolbar actions apply to. */
  get active(): Navigation {
    const tab = this.tab

    return this.focusedSide === 'right' && tab.split ? tab.split : tab
  }

  get isSplit(): boolean {
    return this.tab.split !== null
  }

  toggleSplit() {
    const tab = this.tab

    if (tab.split) {
      tab.split = null
      this.focusedSide = 'left'
      return
    }
    if (tab.view.kind !== 'folder') return

    const pane = new Navigation()
    pane.open(tab.view.path)
    tab.split = pane
    this.focusedSide = 'right'
  }

  /** Ends the split, keeping `keep` as the tab's only pane. */
  collapseSplit(keep: Navigation) {
    const tab = this.tab
    if (!tab.split) return

    if (keep === tab.split) {
      this.#tabs = this.#tabs.map((candidate, index) => (index === this.activeIndex ? keep : candidate))
    }
    tab.split = null
    this.focusedSide = 'left'
  }

  focus(pane: Navigation) {
    const side = pane === this.tab.split ? 'right' : 'left'
    if (side === this.focusedSide) return

    this.focusedSide = side
    // The unfocused pane is not watched, so its listing may be stale.
    pane.reload()
  }

  focusOtherPane() {
    const tab = this.tab
    if (!tab.split) return

    this.focus(this.focusedSide === 'left' ? tab.split : tab)
    void tick().then(() => document.querySelector<HTMLElement>('.home-view__pane--focused .file-list')?.focus())
  }

  get canClose(): boolean {
    return this.#tabs.length > 1
  }

  open(navigation = new Navigation()) {
    if (this.#tabs.length >= MAX_TABS) {
      this.error = `Omafil can open ${MAX_TABS} tabs at once. Close a tab and open the remaining folders again.`
      return false
    }

    this.error = null
    this.#tabs = [...this.#tabs, navigation]
    this.activeIndex = this.#tabs.length - 1
    this.focusedSide = 'left'
    return true
  }

  openPath(path: string) {
    const navigation = new Navigation()
    if (!this.open(navigation)) return false
    navigation.open(path)
    return true
  }

  duplicate() {
    const source = this.active.view
    const copy = new Navigation()

    if (source.kind === 'folder') copy.open(source.path)

    this.open(copy)
  }

  close(index: number) {
    if (!this.canClose || index < 0 || index >= this.#tabs.length) return

    this.#tabs = this.#tabs.filter((_, position) => position !== index)
    if (index < this.activeIndex) this.activeIndex -= 1
    else if (index === this.activeIndex) {
      this.activeIndex = Math.min(index, this.#tabs.length - 1)
      this.focusedSide = 'left'
    }
  }

  /**
   * Refetches every pane in every tab. A setting that changes what a listing
   * contains applies to all of them, not just the one in front.
   */
  reloadAll() {
    for (const tab of this.#tabs) {
      tab.reload()
      tab.split?.reload()
    }
  }

  select(index: number) {
    if (index < 0 || index >= this.#tabs.length) return

    this.activeIndex = index
    // A background tab was not being watched, so its listing may be stale.
    this.active.reload()
  }
}

export const tabs = new Tabs()
