import { Navigation } from './navigation.svelte'

const MAX_TABS = 12

class Tabs {
  #tabs = $state<Navigation[]>([new Navigation()])
  activeIndex = $state(0)

  get all(): Navigation[] {
    return this.#tabs
  }

  get active(): Navigation {
    return this.#tabs[this.activeIndex]
  }

  get canClose(): boolean {
    return this.#tabs.length > 1
  }

  open(navigation = new Navigation()) {
    if (this.#tabs.length >= MAX_TABS) return

    this.#tabs = [...this.#tabs, navigation]
    this.activeIndex = this.#tabs.length - 1
  }

  duplicate() {
    const source = this.active.view
    const copy = new Navigation()

    if (source.kind === 'folder') copy.open(source.path)

    this.open(copy)
  }

  close(index: number) {
    if (!this.canClose) return

    this.#tabs = this.#tabs.filter((_, position) => position !== index)
    this.activeIndex = Math.min(this.activeIndex, this.#tabs.length - 1)
  }

  select(index: number) {
    if (index < 0 || index >= this.#tabs.length) return

    this.activeIndex = index
    // A background tab was not being watched, so its listing may be stale.
    this.active.reload()
  }
}

export const tabs = new Tabs()
