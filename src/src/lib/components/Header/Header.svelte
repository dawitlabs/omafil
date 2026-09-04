<script lang="ts">
  import AddIcon from '@fluentui/svg-icons/icons/add_20_regular.svg?no-inline'
  import ArrowLeftIcon from '@fluentui/svg-icons/icons/arrow_left_20_regular.svg?no-inline'
  import ArrowRightIcon from '@fluentui/svg-icons/icons/arrow_right_20_regular.svg?no-inline'
  import ArrowUpIcon from '@fluentui/svg-icons/icons/arrow_up_20_regular.svg?no-inline'
  import ChevronRightIcon from '@fluentui/svg-icons/icons/chevron_right_20_regular.svg?no-inline'
  import DismissIcon from '@fluentui/svg-icons/icons/dismiss_20_regular.svg?no-inline'
  import FolderIcon from '@fluentui/svg-icons/icons/folder_20_regular.svg?no-inline'
  import HomeIcon from '@fluentui/svg-icons/icons/home_20_regular.svg?no-inline'
  import MicIcon from '@fluentui/svg-icons/icons/mic_20_regular.svg?no-inline'
  import MinimizeIcon from '@fluentui/svg-icons/icons/subtract_20_regular.svg?no-inline'
  import SearchIcon from '@fluentui/svg-icons/icons/search_20_regular.svg?no-inline'
  import SettingsIcon from '@fluentui/svg-icons/icons/settings_20_regular.svg?no-inline'
  import MaximizeIcon from '@fluentui/svg-icons/icons/square_20_regular.svg?no-inline'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { tabs } from '../../tabs.svelte'

  const SEARCH_DELAY_MS = 300

  const appWindow = getCurrentWindow()

  let query = $state('')
  let searchTimer: ReturnType<typeof setTimeout> | null = null

  const active = $derived(tabs.active)

  $effect(() => {
    const view = active.view

    query = view.kind === 'search' ? view.query : ''
  })

  function runSearch(value: string) {
    if (searchTimer) clearTimeout(searchTimer)

    searchTimer = setTimeout(() => {
      if (value.trim()) void active.search(value)
    }, SEARCH_DELAY_MS)
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (!(event.ctrlKey || event.metaKey)) return

    if (event.key.toLowerCase() === 't') {
      event.preventDefault()
      tabs.open()
    } else if (event.key.toLowerCase() === 'w' && tabs.canClose) {
      event.preventDefault()
      tabs.close(tabs.activeIndex)
    }
  }

  async function minimizeWindow() {
    await appWindow.minimize()
  }

  async function toggleMaximize() {
    if (await appWindow.isMaximized()) {
      await appWindow.unmaximize()
      return
    }

    await appWindow.maximize()
  }

  async function closeWindow() {
    await appWindow.close()
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<header class="app-header">
  <div class="app-header__tabbar">
    <div class="app-header__tabs" role="tablist" aria-label="Open tabs">
      {#each tabs.all as tab, index (tab)}
        <div class="app-header__tab" class:app-header__tab--active={index === tabs.activeIndex}>
          <button class="app-header__tab-select" type="button" role="tab" aria-selected={index === tabs.activeIndex} onclick={() => tabs.select(index)}>
            <span class="masked-icon app-header__tab-icon" style="--icon: url({tab.view.kind === 'home' ? HomeIcon : FolderIcon})" aria-hidden="true"></span>
            <span>{tab.label}</span>
          </button>
          {#if tabs.canClose}
            <button class="app-header__tab-close" type="button" aria-label={`Close ${tab.label}`} onclick={() => tabs.close(index)}>
              <span class="masked-icon" style="--icon: url({DismissIcon})" aria-hidden="true"></span>
            </button>
          {/if}
        </div>
      {/each}
    </div>

    <button class="app-header__new-tab" type="button" aria-label="New tab" title="New tab (Ctrl+T)" onclick={() => tabs.open()}>
      <span class="masked-icon app-header__icon" style="--icon: url({AddIcon})" aria-hidden="true"></span>
    </button>

    <div class="app-header__drag-space" data-tauri-drag-region></div>

    <button
      class="app-header__settings-button"
      class:app-header__settings-button--active={active.view.kind === 'settings'}
      type="button"
      aria-label="Settings"
      onclick={() => active.openSettings()}
    >
      <span class="masked-icon app-header__icon" style="--icon: url({SettingsIcon})" aria-hidden="true"></span>
    </button>

    <div class="app-header__window-controls" aria-label="Window controls">
      <button class="app-header__window-button" type="button" aria-label="Minimize" onclick={minimizeWindow}>
        <span class="masked-icon app-header__icon" style="--icon: url({MinimizeIcon})" aria-hidden="true"></span>
      </button>
      <button class="app-header__window-button" type="button" aria-label="Maximize" onclick={toggleMaximize}>
        <span class="masked-icon app-header__icon" style="--icon: url({MaximizeIcon})" aria-hidden="true"></span>
      </button>
      <button class="app-header__window-button app-header__window-button--close" type="button" aria-label="Close" onclick={closeWindow}>
        <span class="masked-icon app-header__icon" style="--icon: url({DismissIcon})" aria-hidden="true"></span>
      </button>
    </div>
  </div>

  <div class="app-header__commandbar">
    <div class="app-header__navigation" aria-label="File navigation controls">
      <button class="app-header__icon-button" type="button" aria-label="Back" onclick={() => active.back()} disabled={!active.canGoBack}>
        <span class="masked-icon app-header__icon" style="--icon: url({ArrowLeftIcon})" aria-hidden="true"></span>
      </button>
      <button class="app-header__icon-button" type="button" aria-label="Forward" onclick={() => active.forward()} disabled={!active.canGoForward}>
        <span class="masked-icon app-header__icon" style="--icon: url({ArrowRightIcon})" aria-hidden="true"></span>
      </button>
      <button class="app-header__icon-button" type="button" aria-label="Up" onclick={() => active.up()} disabled={!active.canGoUp}>
        <span class="masked-icon app-header__icon" style="--icon: url({ArrowUpIcon})" aria-hidden="true"></span>
      </button>
    </div>

    <nav class="app-header__path" aria-label="Breadcrumb">
      <button class="app-header__crumb" type="button" onclick={() => active.goHome()}>
        <span class="masked-icon app-header__tab-icon" style="--icon: url({active.view.kind === 'home' ? HomeIcon : FolderIcon})" aria-hidden="true"></span>
        <span>{active.view.kind === 'home' ? 'Home' : 'Files'}</span>
      </button>

      {#each active.crumbs as crumb (crumb.path)}
        <span class="masked-icon app-header__crumb-separator" style="--icon: url({ChevronRightIcon})" aria-hidden="true"></span>
        <button class="app-header__crumb" type="button" onclick={() => active.open(crumb.path)} aria-current={active.isCurrentPath(crumb.path) ? 'page' : undefined}>
          <span>{crumb.name}</span>
        </button>
      {/each}

      {#if active.view.kind !== 'home' && active.view.kind !== 'folder'}
        <span class="masked-icon app-header__crumb-separator" style="--icon: url({ChevronRightIcon})" aria-hidden="true"></span>
        <span class="app-header__crumb app-header__crumb--static">{active.label}</span>
      {/if}
    </nav>

    <label class="app-header__search">
      <span class="masked-icon app-header__icon" style="--icon: url({SearchIcon})" aria-hidden="true"></span>
      <input
        type="search"
        placeholder={`Search ${active.searchScope}`}
        aria-label={`Search ${active.searchScope}`}
        bind:value={query}
        oninput={() => runSearch(query)}
        onkeydown={(event) => {
          if (event.key === 'Enter') void active.search(query)
          else if (event.key === 'Escape') {
            query = ''
            if (active.view.kind === 'search') active.back()
          }
        }}
      />
      <span class="masked-icon app-header__icon" style="--icon: url({MicIcon})" aria-hidden="true"></span>
    </label>
  </div>
</header>
