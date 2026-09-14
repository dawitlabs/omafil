<script lang="ts">
  import ChevronDownIcon from '@fluentui/svg-icons/icons/chevron_down_12_filled.svg?no-inline'
  import ChevronUpIcon from '@fluentui/svg-icons/icons/chevron_up_12_filled.svg?no-inline'
  import CommandBar from '../CommandBar/CommandBar.svelte'
  import ContextMenu from '../ContextMenu/ContextMenu.svelte'
  import FileRow from '../FileRow/FileRow.svelte'
  import type { ContextMenuItem } from '../ContextMenu/ContextMenu.svelte'
  import { appState } from '../../appState.svelte'
  import { isExtractable } from '../../archives'
  import { fileOperations } from '../../fileOperations.svelte'
  import { formatCount } from '../../format'
  import { tabs } from '../../tabs.svelte'
  import type { DirectoryEntry, EntrySort, Navigation } from '../../navigation.svelte'

  let { navigation }: { navigation: Navigation } = $props()

  type Band = { left: number; top: number; width: number; height: number }

  let list = $state<HTMLElement | null>(null)
  let activeIndex = $state(0)
  let band = $state<Band | null>(null)
  let menu = $state<{ x: number; y: number; items: ContextMenuItem[] } | null>(null)
  let bandOrigin: { x: number; y: number } | null = null
  let typeAhead = ''
  let typeAheadTimer: ReturnType<typeof setTimeout> | null = null

  const entries = $derived(navigation.entries)
  const hiddenCount = $derived(navigation.hiddenCount)

  // Sorting is a property of a directory listing; search and tag results come
  // back in walk order, so their headers are labels rather than controls.
  const isSortable = $derived(navigation.view.kind === 'folder')

  const columns = $derived(
    isSortable
      ? [
          { sort: 'name' as EntrySort, label: 'Name', className: 'file-row__name' },
          { sort: 'modified' as EntrySort, label: 'Date modified', className: 'file-row__modified' },
          { sort: 'type' as EntrySort, label: 'Type', className: 'file-row__kind' },
          { sort: 'size' as EntrySort, label: 'Size', className: 'file-row__size' },
        ]
      : [
          { sort: 'name' as EntrySort, label: 'Name', className: 'file-row__name' },
          { sort: 'type' as EntrySort, label: 'Folder', className: 'file-row__kind' },
          { sort: 'modified' as EntrySort, label: 'Date modified', className: 'file-row__modified' },
          { sort: 'size' as EntrySort, label: 'Size', className: 'file-row__size' },
        ],
  )

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
        { kind: 'action', label: 'Open', onSelect: () => navigation.openEntry(entry) },
        ...(entry.entryType === 'file' ? [{ kind: 'action', label: 'Edit', onSelect: () => navigation.openInEditor(entry.path) } as ContextMenuItem, { kind: 'action', label: 'Open with…', onSelect: () => fileOperations.showOpenWith(entry.path) } as ContextMenuItem] : []),
        { kind: 'action', label: entry.entryType === 'directory' ? 'Open in terminal' : 'Open folder in terminal', shortcut: 'F4', onSelect: () => navigation.openTerminal(entry.path) },
        { kind: 'separator' },
        { kind: 'action', label: 'Cut', shortcut: 'Ctrl+X', onSelect: () => fileOperations.cutSelection() },
        { kind: 'action', label: 'Copy', shortcut: 'Ctrl+C', onSelect: () => fileOperations.copySelection() },
        { kind: 'separator' },
        ...(fileOperations.selectedPaths.length > 1 ? [{ kind: 'action', label: 'Compress to ZIP', onSelect: () => fileOperations.compressSelection() } as ContextMenuItem] : []),
        { kind: 'action', label: 'Extract here', disabled: !isExtractable(entry.name), onSelect: () => fileOperations.extractSelection() },
        { kind: 'separator' },
        {
          kind: 'action',
          label: 'Properties',
          shortcut: 'Alt+Enter',
          disabled: fileOperations.selectedPaths.length !== 1,
          onSelect: () => fileOperations.showProperties(entry.path),
        },
        {
          kind: 'action',
          label: fileOperations.selectedPaths.length > 1 && fileOperations.isSelected(entry.path) ? `Rename ${fileOperations.selectedPaths.length} items…` : 'Rename',
          shortcut: 'F2',
          onSelect: () => fileOperations.startRenaming(entry.path),
        },
        { kind: 'action', label: 'Move to trash', shortcut: 'Del', onSelect: () => fileOperations.deleteSelection() },
        { kind: 'action', label: 'Delete permanently', shortcut: 'Shift+Del', onSelect: () => fileOperations.requestPermanentDelete() },
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
        { kind: 'action', label: 'Open in terminal', shortcut: 'F4', disabled: !fileOperations.directoryPath, onSelect: () => navigation.openTerminal() },
        { kind: 'action', label: 'Refresh', onSelect: () => navigation.reload() },
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

  function selectByTyping(character: string) {
    const lowerCharacter = character.toLocaleLowerCase()
    typeAhead = typeAhead === lowerCharacter ? lowerCharacter : `${typeAhead}${lowerCharacter}`

    if (typeAheadTimer) clearTimeout(typeAheadTimer)
    typeAheadTimer = setTimeout(() => {
      typeAhead = ''
      typeAheadTimer = null
    }, 900)

    const ordered = [...entries.slice(activeIndex + 1), ...entries.slice(0, activeIndex + 1)]
    const match = ordered.find((candidate) => candidate.name.toLocaleLowerCase().startsWith(typeAhead))

    if (match) moveSelection(entries.indexOf(match))
  }

  let isAwaitingG = false

  function handleKeydown(event: KeyboardEvent) {
    if (fileOperations.renamingPath) return

    const entry = entries[activeIndex]

    if ((event.ctrlKey || event.metaKey) && event.shiftKey && event.key.toLowerCase() === 'n') {
      event.preventDefault()
      event.stopPropagation()
      void fileOperations.createFolder()
      return
    }

    const moves: Record<string, number> = {
      ArrowDown: activeIndex + 1,
      ArrowUp: activeIndex - 1,
      Home: 0,
      End: entries.length - 1,
    }

    if (appState.settings.vimKeys && !event.altKey) {
      const isSecondG = event.key === 'g' && isAwaitingG
      isAwaitingG = event.key === 'g' && !isSecondG
      Object.assign(moves, { j: activeIndex + 1, k: activeIndex - 1, G: entries.length - 1, ...(isSecondG ? { g: 0 } : {}) })

      if (event.key === 'h') {
        event.preventDefault()
        navigation.up()
        return
      }
      if (event.key === 'l' && entry) {
        event.preventDefault()
        void navigation.openEntry(entry)
        return
      }
      if (event.key === 'g' || event.key === '/') return
    }

    if (event.key in moves) {
      event.preventDefault()
      moveSelection(moves[event.key])
      return
    }

    if (event.key === 'F5') {
      event.preventDefault()
      navigation.reload()
    }
    else if (event.key === 'Backspace') {
      event.preventDefault()
      navigation.up()
    }
    else if (event.altKey && event.key === 'Enter') fileOperations.showProperties()
    else if (event.key === 'Enter' && entry) void navigation.openEntry(entry)
    else if (event.key === 'F2') fileOperations.startRenaming()
    else if (event.key === 'F4') void navigation.openTerminal()
    else if (event.key === 'F6') tabs.focusOtherPane()
    else if (event.shiftKey && event.key === 'Delete') fileOperations.requestPermanentDelete()
    else if (event.key === 'Delete') void fileOperations.deleteSelection()
    else if (event.key === 'Escape') fileOperations.clearSelection()
    else if (event.key.length === 1 && !event.altKey && !appState.settings.vimKeys) {
      event.preventDefault()
      selectByTyping(event.key)
    }
  }

  function startDrag(entry: DirectoryEntry, event: DragEvent) {
    if (!fileOperations.isSelected(entry.path)) fileOperations.selectOnly(entry.path)
    event.dataTransfer?.setData('application/x-omafil-paths', JSON.stringify(fileOperations.selectedPaths))
    if (event.dataTransfer) event.dataTransfer.effectAllowed = 'copyMove'
  }

  function dropOnFolder(entry: DirectoryEntry, event: DragEvent) {
    if (entry.entryType !== 'directory') return

    const source = event.dataTransfer?.getData('application/x-omafil-paths')
    if (!source) return

    try {
      const paths = JSON.parse(source) as string[]
      void fileOperations.movePathsTo(paths, entry.path, event.ctrlKey || event.metaKey)
    } catch {
      // Drops from other applications are intentionally left to their native
      // import workflow; this slice transfers items already in Omafil.
    }
  }

  /// The clipboard shortcuts live on the window because the folder list is not always
  /// the focused element, and a text field has first claim on them when it is.
  function isTypingInto(target: EventTarget | null): boolean {
    return target instanceof HTMLElement && (target.isContentEditable || ['INPUT', 'TEXTAREA'].includes(target.tagName))
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    // Split view mounts one of these per pane, so only the focused pane may act on a window key.
    if (tabs.active !== navigation) return

    if ((event.ctrlKey || event.metaKey) && !event.shiftKey && !event.altKey && !isTypingInto(event.target) && !fileOperations.renamingPath) {
      const shortcuts: Record<string, () => void> = {
        a: () => fileOperations.selectAll(),
        z: () => void fileOperations.undo(),
        c: () => fileOperations.copySelection(),
        x: () => fileOperations.cutSelection(),
        v: () => void fileOperations.paste(),
        '1': () => (fileOperations.viewMode = 'details'),
        '2': () => (fileOperations.viewMode = 'icons'),
        '3': () => (fileOperations.viewMode = 'preview'),
      }
      const shortcut = shortcuts[event.key.toLowerCase()]

      if (shortcut) {
        event.preventDefault()
        shortcut()
        return
      }
    }

    if (!event.altKey) return

    if (event.key === 'ArrowLeft') navigation.back()
    else if (event.key === 'ArrowRight') navigation.forward()
    else if (event.key === 'ArrowUp') navigation.up()
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} onfocus={() => tabs.active === navigation && void fileOperations.syncFromDesktopClipboard()} />

<CommandBar />

{#if fileOperations.error}
  <div class="file-list__error" role="alert">
    <p>{fileOperations.error}</p>
    <button class="home-view__retry" type="button" onclick={() => (fileOperations.error = null)}>Dismiss</button>
  </div>
{/if}

{#if navigation.missingTagged > 0}
  <p class="file-list__notice">
    {formatCount(navigation.missingTagged)}
    tagged {navigation.missingTagged === 1 ? 'item is' : 'items are'} not reachable right now, so they are not listed.
  </p>
{/if}

{#if navigation.isSearchTruncated}
  <p class="file-list__notice">Showing the first {formatCount(entries.length)} matches. Narrow the search to see fewer.</p>
{/if}

{#if hiddenCount > 0}
  <p class="file-list__notice">
    Showing the first {formatCount(entries.length)} of {formatCount(navigation.total)} items, sorted by {navigation.sort}.
  </p>
{/if}

{#if fileOperations.viewMode === 'details'}
  <div class="file-list__columns" role="presentation">
    <span></span>
    {#each columns as column (column.label)}
    {#if isSortable}
      <button
        class="file-list__column {column.className}"
        type="button"
        data-sort={navigation.sort === column.sort ? (navigation.descending ? 'descending' : 'ascending') : 'none'}
        aria-label={navigation.sort === column.sort
          ? `Sort by ${column.label}, currently ${navigation.descending ? 'descending' : 'ascending'}`
          : `Sort by ${column.label}`}
        onclick={() => navigation.sortBy(column.sort)}
      >
        <span>{column.label}</span>
        {#if navigation.sort === column.sort}
          <span class="masked-icon file-list__sort-arrow" style="--icon: url({navigation.descending ? ChevronDownIcon : ChevronUpIcon})" aria-hidden="true"></span>
        {/if}
      </button>
    {:else}
      <span class="file-list__column file-list__column--static {column.className}">{column.label}</span>
    {/if}
    {/each}
  </div>
{/if}

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  bind:this={list}
  class="file-list"
  class:file-list--icons={fileOperations.viewMode === 'icons'}
  class:file-list--preview={fileOperations.viewMode === 'preview'}
  role="listbox"
  aria-multiselectable="true"
  aria-label="Folder contents"
  tabindex="-1"
  onkeydown={handleKeydown}
  onfocusin={() => tabs.focus(navigation)}
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
      showFolder={!isSortable}
      previewMode={fileOperations.viewMode === 'preview'}
      onactivate={(event) => activateRow(entry, index, event)}
      onopen={() => navigation.openEntry(entry)}
      onmenu={(event) => openMenuForRow(entry, index, event)}
      ondragstart={(event) => startDrag(entry, event)}
      ondragover={(event) => {
        if (entry.entryType === 'directory') event.preventDefault()
      }}
      ondrop={(event) => dropOnFolder(entry, event)}
    />
  {/each}

  {#if entries.length === 0}
    <p class="file-list__empty">This folder is empty.</p>
  {/if}

  {#if band}
    <div class="file-list__band" style="left: {band.left}px; top: {band.top}px; width: {band.width}px; height: {band.height}px" aria-hidden="true"></div>
  {/if}
</div>

{#if fileOperations.pendingTransfer}
  <div class="file-conflict__backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && fileOperations.cancelPendingTransfer()}>
    <dialog open class="file-conflict" aria-labelledby="file-conflict-title">
      <h2 id="file-conflict-title">Items already exist here</h2>
      <p>{formatCount(fileOperations.pendingTransfer.conflicts.length)} item{fileOperations.pendingTransfer.conflicts.length === 1 ? '' : 's'} have the same name in the destination.</p>
      <ul>{#each fileOperations.pendingTransfer.conflicts.slice(0, 4) as conflict (conflict.destinationPath)}<li>{conflict.name}</li>{/each}</ul>
      <div class="file-conflict__actions">
        <button type="button" onclick={() => fileOperations.cancelPendingTransfer()}>Cancel</button>
        <button type="button" onclick={() => fileOperations.resolvePendingTransfer('skip')}>Skip existing</button>
        <button type="button" onclick={() => fileOperations.resolvePendingTransfer('rename')}>Keep both</button>
        <button class="file-conflict__replace" type="button" onclick={() => fileOperations.resolvePendingTransfer('replace')}>Replace</button>
      </div>
    </dialog>
  </div>
{/if}

{#if fileOperations.pendingPermanentDelete}
  <div class="file-conflict__backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && fileOperations.cancelPermanentDelete()}>
    <dialog open class="file-conflict" aria-labelledby="permanent-delete-title">
      <h2 id="permanent-delete-title">Permanently delete?</h2>
      <p>{formatCount(fileOperations.pendingPermanentDelete.length)} selected item{fileOperations.pendingPermanentDelete.length === 1 ? '' : 's'} will be deleted without being moved to the trash.</p>
      <div class="file-conflict__actions">
        <button type="button" onclick={() => fileOperations.cancelPermanentDelete()}>Cancel</button>
        <button class="file-conflict__replace" type="button" onclick={() => fileOperations.confirmPermanentDelete()}>Delete permanently</button>
      </div>
    </dialog>
  </div>
{/if}

{#if fileOperations.pendingArchivePaths}
  <div class="file-conflict__backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && fileOperations.cancelArchive()}>
    <dialog open class="file-conflict file-archive-dialog" aria-labelledby="archive-title">
      <h2 id="archive-title">Create ZIP archive</h2>
      <p>{fileOperations.pendingArchivePaths.length} selected item{fileOperations.pendingArchivePaths.length === 1 ? '' : 's'} will be compressed into a ZIP file.</p>
      <label class="file-archive-dialog__field" for="archive-name">Archive name<!-- svelte-ignore a11y_autofocus --><input id="archive-name" autofocus bind:value={fileOperations.archiveDraft} onkeydown={(event) => { if (event.key === 'Enter') fileOperations.createArchive(); else if (event.key === 'Escape') fileOperations.cancelArchive() }} /></label>
      <div class="file-conflict__actions"><button type="button" onclick={() => fileOperations.cancelArchive()}>Cancel</button><button class="file-conflict__replace" type="button" disabled={!fileOperations.archiveDraft.trim()} onclick={() => fileOperations.createArchive()}>Create</button></div>
    </dialog>
  </div>
{/if}

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} items={menu.items} onclose={() => (menu = null)} />
{/if}
