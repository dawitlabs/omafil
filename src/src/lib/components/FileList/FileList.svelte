<script lang="ts">
  import ChevronDownIcon from '@fluentui/svg-icons/icons/chevron_down_12_filled.svg?no-inline'
  import ChevronUpIcon from '@fluentui/svg-icons/icons/chevron_up_12_filled.svg?no-inline'
  import CommandBar from '../CommandBar/CommandBar.svelte'
  import ContextMenu from '../ContextMenu/ContextMenu.svelte'
  import FileRow from '../FileRow/FileRow.svelte'
  import type { ContextMenuItem } from '../ContextMenu/ContextMenu.svelte'
  import { appState } from '../../appState.svelte'
  import { fileOperations } from '../../fileOperations.svelte'
  import { formatCount } from '../../format'
  import { tabs } from '../../tabs.svelte'
  import type { DirectoryEntry, EntrySort } from '../../navigation.svelte'

  type Band = { left: number; top: number; width: number; height: number }

  let list = $state<HTMLElement | null>(null)
  let activeIndex = $state(0)
  let band = $state<Band | null>(null)
  let menu = $state<{ x: number; y: number; items: ContextMenuItem[] } | null>(null)
  let bandOrigin: { x: number; y: number } | null = null

  const entries = $derived(tabs.active.entries)
  const hiddenCount = $derived(tabs.active.hiddenCount)

  const columns: Array<{ sort: EntrySort; label: string; className: string }> = [
    { sort: 'name', label: 'Name', className: 'file-row__name' },
    { sort: 'modified', label: 'Date modified', className: 'file-row__modified' },
    { sort: 'type', label: 'Type', className: 'file-row__kind' },
    { sort: 'size', label: 'Size', className: 'file-row__size' },
  ]

  function focusRow(index: number) {
    activeIndex = Math.max(0, Math.min(index, entries.length - 1))
    const row = list?.querySelector<HTMLElement>(`[data-index="${activeIndex}"]`)
    row?.focus()
    row?.scrollIntoView({ block: 'nearest' })
  }

  function activateRow(entry: DirectoryEntry, index: number, event: MouseEvent) {
    activeIndex = index

    if (event.shiftKey) {
      fileOperations.extendTo(entry.path)
      return
    }

    if (event.ctrlKey || event.metaKey) {
      fileOperations.toggle(entry.path)
      return
    }

    if (!fileOperations.isSelected(entry.path)) fileOperations.selectOnly(entry.path)
  }

  function openMenuForRow(entry: DirectoryEntry, index: number, event: MouseEvent) {
    activateRow(entry, index, event)

    menu = {
      x: event.clientX,
      y: event.clientY,
      items: [
        { kind: 'action', label: 'Open', onSelect: () => tabs.active.openEntry(entry) },
        { kind: 'separator' },
        { kind: 'action', label: 'Cut', shortcut: 'Ctrl+X', onSelect: () => fileOperations.cutSelection() },
        { kind: 'action', label: 'Copy', shortcut: 'Ctrl+C', onSelect: () => fileOperations.copySelection() },
        { kind: 'separator' },
        {
          kind: 'action',
          label: 'Rename',
          shortcut: 'F2',
          disabled: fileOperations.selectedPaths.length !== 1,
          onSelect: () => fileOperations.startRenaming(entry.path),
        },
        { kind: 'action', label: 'Move to trash', shortcut: 'Del', onSelect: () => fileOperations.deleteSelection() },
        { kind: 'separator' },
        {
          kind: 'action',
          label: appState.isPinned(entry.path) ? 'Unpin from sidebar' : 'Pin to sidebar',
          disabled: entry.entryType !== 'directory',
          onSelect: () => appState.togglePin(entry.path),
        },
        ...(appState.tags.length > 0 ? [{ kind: 'heading', label: 'Tags' } as ContextMenuItem] : []),
        ...appState.tags.map(
          (tag): ContextMenuItem => ({
            kind: 'toggle',
            label: tag.label,
            checked: appState.tagsFor(entry.path).some((assigned) => assigned.id === tag.id),
            onSelect: () => appState.toggleTag(entry.path, tag.id),
          }),
        ),
      ],
    }
  }

  function openMenuForBackground(event: MouseEvent) {
    fileOperations.clearSelection()

    menu = {
      x: event.clientX,
      y: event.clientY,
      items: [
        {
          kind: 'action',
          label: 'New folder',
          shortcut: 'Ctrl+Shift+N',
          disabled: !fileOperations.directoryPath,
          onSelect: () => fileOperations.createFolder(),
        },
        { kind: 'separator' },
        { kind: 'action', label: 'Paste', shortcut: 'Ctrl+V', disabled: !fileOperations.canPaste, onSelect: () => fileOperations.paste() },
        { kind: 'separator' },
        { kind: 'action', label: 'Refresh', onSelect: () => tabs.active.reload() },
      ],
    }
  }

  function startBandSelect(event: PointerEvent) {
    if (event.button !== 0 || !list || event.target !== list) return

    fileOperations.clearSelection()
    bandOrigin = { x: event.clientX, y: event.clientY }
    list.setPointerCapture(event.pointerId)
  }

  function updateBandSelect(event: PointerEvent) {
    if (!bandOrigin || !list) return

    const bounds = list.getBoundingClientRect()
    const left = Math.min(bandOrigin.x, event.clientX)
    const top = Math.min(bandOrigin.y, event.clientY)
    const right = Math.max(bandOrigin.x, event.clientX)
    const bottom = Math.max(bandOrigin.y, event.clientY)

    band = { left: left - bounds.left, top: top - bounds.top, width: right - left, height: bottom - top }

    const covered = [...list.querySelectorAll<HTMLElement>('[data-path]')]
      .filter((row) => {
        const rect = row.getBoundingClientRect()
        return rect.bottom > top && rect.top < bottom && rect.right > left && rect.left < right
      })
      .map((row) => row.dataset.path ?? '')

    fileOperations.selectWithin(covered)
  }

  function endBandSelect(event: PointerEvent) {
    if (!bandOrigin) return

    bandOrigin = null
    band = null
    list?.releasePointerCapture(event.pointerId)
  }

  function moveSelection(index: number) {
    focusRow(index)
    const entry = entries[activeIndex]

    if (entry) fileOperations.selectOnly(entry.path)
  }

  function handleKeydown(event: KeyboardEvent) {
    if (fileOperations.renamingPath) return

    const entry = entries[activeIndex]

    if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key.toLowerCase() === 'n') {
      event.preventDefault()
      void fileOperations.createFolder()
      return
    }

    if (event.ctrlKey || event.metaKey) {
      const shortcuts: Record<string, () => void> = {
        a: () => fileOperations.selectAll(),
        c: () => fileOperations.copySelection(),
        x: () => fileOperations.cutSelection(),
        v: () => void fileOperations.paste(),
      }
      const shortcut = shortcuts[event.key.toLowerCase()]

      if (shortcut) {
        event.preventDefault()
        shortcut()
      }
      return
    }

    const moves: Record<string, number> = {
      ArrowDown: activeIndex + 1,
      ArrowUp: activeIndex - 1,
      Home: 0,
      End: entries.length - 1,
    }

    if (event.key in moves) {
      event.preventDefault()
      moveSelection(moves[event.key])
      return
    }

    if (event.key === 'Enter' && entry) void tabs.active.openEntry(entry)
    else if (event.key === 'F2') fileOperations.startRenaming()
    else if (event.key === 'Delete') void fileOperations.deleteSelection()
    else if (event.key === 'Escape') fileOperations.clearSelection()
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (!event.altKey) return

    if (event.key === 'ArrowLeft') tabs.active.back()
    else if (event.key === 'ArrowRight') tabs.active.forward()
    else if (event.key === 'ArrowUp') tabs.active.up()
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<CommandBar />

{#if fileOperations.error}
  <div class="file-list__error" role="alert">
    <p>{fileOperations.error}</p>
    <button class="home-view__retry" type="button" onclick={() => (fileOperations.error = null)}>Dismiss</button>
  </div>
{/if}

{#if tabs.active.isSearchTruncated}
  <p class="file-list__notice">Showing the first {formatCount(entries.length)} matches. Narrow the search to see fewer.</p>
{/if}

{#if hiddenCount > 0}
  <p class="file-list__notice">
    Showing the first {formatCount(entries.length)} of {formatCount(tabs.active.total)} items, sorted by {tabs.active.sort}.
  </p>
{/if}

<div class="file-list__columns" role="presentation">
  <span></span>
  {#each columns as column (column.sort)}
    <button
      class="file-list__column {column.className}"
      type="button"
      data-sort={tabs.active.sort === column.sort ? (tabs.active.descending ? 'descending' : 'ascending') : 'none'}
      aria-label={tabs.active.sort === column.sort
        ? `Sort by ${column.label}, currently ${tabs.active.descending ? 'descending' : 'ascending'}`
        : `Sort by ${column.label}`}
      onclick={() => tabs.active.sortBy(column.sort)}
    >
      <span>{column.label}</span>
      {#if tabs.active.sort === column.sort}
        <span class="masked-icon file-list__sort-arrow" style="--icon: url({tabs.active.descending ? ChevronDownIcon : ChevronUpIcon})" aria-hidden="true"></span>
      {/if}
    </button>
  {/each}
</div>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  bind:this={list}
  class="file-list"
  role="listbox"
  aria-multiselectable="true"
  aria-label="Folder contents"
  tabindex="-1"
  onkeydown={handleKeydown}
  onpointerdown={startBandSelect}
  onpointermove={updateBandSelect}
  onpointerup={endBandSelect}
  oncontextmenu={(event) => {
    if (event.target === list) {
      event.preventDefault()
      openMenuForBackground(event)
    }
  }}
>
  {#each entries as entry, index (entry.path)}
    <FileRow
      {entry}
      {index}
      isActive={index === activeIndex}
      onactivate={(event) => activateRow(entry, index, event)}
      onopen={() => tabs.active.openEntry(entry)}
      onmenu={(event) => openMenuForRow(entry, index, event)}
    />
  {/each}

  {#if entries.length === 0}
    <p class="file-list__empty">This folder is empty.</p>
  {/if}

  {#if band}
    <div class="file-list__band" style="left: {band.left}px; top: {band.top}px; width: {band.width}px; height: {band.height}px" aria-hidden="true"></div>
  {/if}
</div>

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} items={menu.items} onclose={() => (menu = null)} />
{/if}
