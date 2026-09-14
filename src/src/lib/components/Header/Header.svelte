<script lang="ts">
  import AddIcon from '@fluentui/svg-icons/icons/add_20_regular.svg?no-inline'
  import ArrowLeftIcon from '@fluentui/svg-icons/icons/arrow_left_20_regular.svg?no-inline'
  import ArrowRightIcon from '@fluentui/svg-icons/icons/arrow_right_20_regular.svg?no-inline'
  import ArrowUpIcon from '@fluentui/svg-icons/icons/arrow_up_20_regular.svg?no-inline'
  import ChevronRightIcon from '@fluentui/svg-icons/icons/chevron_right_20_regular.svg?no-inline'
  import DismissIcon from '@fluentui/svg-icons/icons/dismiss_20_regular.svg?no-inline'
  import FolderIcon from '@fluentui/svg-icons/icons/folder_20_regular.svg?no-inline'
  import HomeIcon from '@fluentui/svg-icons/icons/home_20_regular.svg?no-inline'
  import SearchIcon from '@fluentui/svg-icons/icons/search_20_regular.svg?no-inline'
  import SettingsIcon from '@fluentui/svg-icons/icons/settings_20_regular.svg?no-inline'
  import SplitIcon from '@fluentui/svg-icons/icons/layout_column_two_20_regular.svg?no-inline'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import { tabs } from '../../tabs.svelte'
  import { appState } from '../../appState.svelte'
  import { fileOperations } from '../../fileOperations.svelte'

  const SEARCH_DELAY_MS = 300

  const appWindow = getCurrentWindow()

  let query = $state('')
  let address = $state('')
  let editingAddress = $state(false)
  let searchTimer: ReturnType<typeof setTimeout> | null = null
  let searchInput = $state<HTMLInputElement | null>(null)

  const active = $derived(tabs.active)

  $effect(() => {
    const view = active.view

    query = view.kind === 'search' ? view.query : ''
    address = view.kind === 'folder' ? view.path : ''
  })

  function runSearch(value: string) {
    if (searchTimer) clearTimeout(searchTimer)

    searchTimer = setTimeout(() => {
      if (value.trim()) void active.search(value)
    }, SEARCH_DELAY_MS)
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    const target = event.target as HTMLElement | null
    if (target?.matches('input, textarea, select, [contenteditable="true"]')) return

    if (event.key === 'F5') {
      event.preventDefault()
      active.reload()
      return
    }

    if (event.key === 'F6') {
      event.preventDefault()
      tabs.focusOtherPane()
      return
    }

    if (event.key === '/' && appState.settings.vimKeys) {
      event.preventDefault()
      searchInput?.focus()
      searchInput?.select()
      return
    }

    if (!(event.ctrlKey || event.metaKey)) return

    if (event.shiftKey && event.key.toLowerCase() === 'n' && fileOperations.directoryPath) {
      event.preventDefault()
      void fileOperations.createFolder()
    } else if (event.key.toLowerCase() === 't') {
      event.preventDefault()
      tabs.open()
    } else if (event.key.toLowerCase() === 'w' && tabs.canClose) {
      event.preventDefault()
      tabs.close(tabs.activeIndex)
    } else if (event.key.toLowerCase() === 'l') {
      event.preventDefault()
      editingAddress = true
    } else if (event.key.toLowerCase() === 'f' || event.key.toLowerCase() === 'e') {
      event.preventDefault()
      searchInput?.focus()
      searchInput?.select()
    } else if (event.key.toLowerCase() === 'r') {
      event.preventDefault()
      active.reload()
    }
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
      class:app-header__settings-button--active={tabs.isSplit}
      type="button"
      aria-pressed={tabs.isSplit}
      aria-label={tabs.isSplit ? 'Close split pane' : 'Split into two panes'}
      title={tabs.isSplit ? 'Close split pane' : 'Split into two panes (F6 switches panes)'}
      disabled={!tabs.isSplit && active.view.kind !== 'folder'}
      onclick={() => tabs.toggleSplit()}
    >
      <span class="masked-icon app-header__icon" style="--icon: url({SplitIcon})" aria-hidden="true"></span>
    </button>
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

    {#if editingAddress}
      <form class="app-header__address" onsubmit={(event) => {
        event.preventDefault()
        const path = address.trim()
        if (path) active.open(path)
        editingAddress = false
      }}>
        <!-- svelte-ignore a11y_autofocus -->
        <input autofocus aria-label="Folder path" bind:value={address} onkeydown={(event) => {
          if (event.key === 'Escape') editingAddress = false
        }} />
      </form>
    {:else}
    <nav class="app-header__path" aria-label="Breadcrumb" ondblclick={() => (editingAddress = true)}>
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
    {/if}

    <label class="app-header__search">
      <span class="masked-icon app-header__icon" style="--icon: url({SearchIcon})" aria-hidden="true"></span>
      <input
        type="search"
        bind:this={searchInput}
        placeholder={`Search ${active.searchScope} · type:image size:>10mb`}
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
    </label>
  </div>
</header>
