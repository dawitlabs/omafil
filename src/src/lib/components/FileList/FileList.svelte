<script lang="ts">
  import ContextMenu from '../ContextMenu/ContextMenu.svelte'
  import type { ContextMenuItem } from '../ContextMenu/ContextMenu.svelte'
  import { fileIcon, folderIcon } from '../../fileIcons'
  import { fileOperations } from '../../fileOperations.svelte'
  import { navigation } from '../../navigation.svelte'
  import type { DirectoryEntry } from '../../navigation.svelte'

  type Band = { left: number; top: number; width: number; height: number }

  let list = $state<HTMLElement | null>(null)
  let activeIndex = $state(0)
  let band = $state<Band | null>(null)
  let menu = $state<{ x: number; y: number; items: ContextMenuItem[] } | null>(null)
  let renameDraft = $state('')
  let renameSource = $state<string | null>(null)
  let bandOrigin: { x: number; y: number } | null = null

  const entries = $derived(navigation.listing?.entries ?? [])

  $effect(() => {
    const path = fileOperations.renamingPath

    if (!path) {
      renameSource = null
      return
    }

    const entry = entries.find((candidate) => candidate.path === path)

    if (entry && renameSource !== path) {
      renameSource = path
      renameDraft = entry.name
    }
  })

  function iconFor(entry: DirectoryEntry) {
    return entry.entryType === 'directory' ? folderIcon : fileIcon(entry.name)
  }

  function focusRow(index: number) {
    activeIndex = Math.max(0, Math.min(index, entries.length - 1))
    const row = list?.querySelector<HTMLElement>(`[data-index="${activeIndex}"]`)
    row?.focus()
    row?.scrollIntoView({ block: 'nearest' })
  }

  function handleRowPointerDown(entry: DirectoryEntry, index: number, event: MouseEvent) {
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

  function beginRenaming(path: string) {
    renameDraft = entries.find((entry) => entry.path === path)?.name ?? ''
    fileOperations.startRenaming(path)
  }

  function commitRename(path: string) {
    void fileOperations.rename(path, renameDraft)
  }

  function openMenuForRow(entry: DirectoryEntry, index: number, event: MouseEvent) {
    handleRowPointerDown(entry, index, event)

    const selectedCount = fileOperations.selectedPaths.length

    menu = {
      x: event.clientX,
      y: event.clientY,
      items: [
        { kind: 'action', label: 'Open', onSelect: () => navigation.openEntry(entry) },
        { kind: 'separator' },
        { kind: 'action', label: 'Cut', shortcut: 'Ctrl+X', onSelect: () => fileOperations.cutSelection() },
        { kind: 'action', label: 'Copy', shortcut: 'Ctrl+C', onSelect: () => fileOperations.copySelection() },
        { kind: 'separator' },
        { kind: 'action', label: 'Rename', shortcut: 'F2', disabled: selectedCount !== 1, onSelect: () => beginRenaming(entry.path) },
        { kind: 'action', label: 'Move to trash', shortcut: 'Del', onSelect: () => fileOperations.deleteSelection() },
      ],
    }
  }

  function openMenuForBackground(event: MouseEvent) {
    fileOperations.clearSelection()

    menu = {
      x: event.clientX,
      y: event.clientY,
      items: [
        { kind: 'action', label: 'New folder', shortcut: 'Ctrl+Shift+N', onSelect: () => fileOperations.createFolder() },
        { kind: 'separator' },
        { kind: 'action', label: 'Paste', shortcut: 'Ctrl+V', disabled: !fileOperations.canPaste, onSelect: () => fileOperations.paste() },
        { kind: 'separator' },
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

  function handleKeydown(event: KeyboardEvent) {
    if (fileOperations.renamingPath) return

    const isModified = event.ctrlKey || event.metaKey
    const entry = entries[activeIndex]

    if (isModified && event.shiftKey && event.key.toLowerCase() === 'n') {
      event.preventDefault()
      void fileOperations.createFolder()
      return
    }

    if (isModified) {
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

    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault()
        focusRow(activeIndex + 1)
        if (entries[activeIndex]) fileOperations.selectOnly(entries[activeIndex].path)
        break
      case 'ArrowUp':
        event.preventDefault()
        focusRow(activeIndex - 1)
        if (entries[activeIndex]) fileOperations.selectOnly(entries[activeIndex].path)
        break
      case 'Home':
        event.preventDefault()
        focusRow(0)
        if (entries[0]) fileOperations.selectOnly(entries[0].path)
        break
      case 'End':
        event.preventDefault()
        focusRow(entries.length - 1)
        if (entries[activeIndex]) fileOperations.selectOnly(entries[activeIndex].path)
        break
      case 'Enter':
        if (entry) void navigation.openEntry(entry)
        break
      case 'F2':
        if (fileOperations.selectedPaths.length === 1) beginRenaming(fileOperations.selectedPaths[0])
        break
      case 'Delete':
        void fileOperations.deleteSelection()
        break
      case 'Escape':
        fileOperations.clearSelection()
        break
    }
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.altKey && event.key === 'ArrowLeft') navigation.back()
    else if (event.altKey && event.key === 'ArrowRight') navigation.forward()
    else if (event.altKey && event.key === 'ArrowUp') navigation.up()
  }
</script>

<svelte:window onkeydown={handleWindowKeydown} />

<div class="file-commands" role="toolbar" aria-label="File commands">
  <button class="file-commands__button" type="button" onclick={() => fileOperations.createFolder()}>New folder</button>
  <span class="file-commands__divider" aria-hidden="true"></span>
  <button class="file-commands__button" type="button" disabled={fileOperations.selectedPaths.length === 0} onclick={() => fileOperations.cutSelection()}>Cut</button>
  <button class="file-commands__button" type="button" disabled={fileOperations.selectedPaths.length === 0} onclick={() => fileOperations.copySelection()}>Copy</button>
  <button class="file-commands__button" type="button" disabled={!fileOperations.canPaste} onclick={() => fileOperations.paste()}>Paste</button>
  <span class="file-commands__divider" aria-hidden="true"></span>
  <button
    class="file-commands__button"
    type="button"
    disabled={fileOperations.selectedPaths.length !== 1}
    onclick={() => beginRenaming(fileOperations.selectedPaths[0])}
  >
    Rename
  </button>
  <button class="file-commands__button" type="button" disabled={fileOperations.selectedPaths.length === 0} onclick={() => fileOperations.deleteSelection()}>Delete</button>
</div>

{#if fileOperations.error}
  <div class="file-list__error" role="alert">
    <p>{fileOperations.error}</p>
    <button class="home-view__retry" type="button" onclick={() => (fileOperations.error = null)}>Dismiss</button>
  </div>
{/if}

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
    <div
      class="file-row"
      class:file-row--selected={fileOperations.isSelected(entry.path)}
      class:file-row--cut={fileOperations.clipboardMode === 'cut' && fileOperations.clipboardPaths.includes(entry.path)}
      data-path={entry.path}
      data-index={index}
      role="option"
      aria-selected={fileOperations.isSelected(entry.path)}
      tabindex={index === activeIndex ? 0 : -1}
      onpointerdown={(event) => handleRowPointerDown(entry, index, event)}
      ondblclick={() => navigation.openEntry(entry)}
      oncontextmenu={(event) => {
        event.preventDefault()
        openMenuForRow(entry, index, event)
      }}
    >
      <span class="masked-icon file-row__icon" style="--icon: url({iconFor(entry).icon}); color: {iconFor(entry).tone}" aria-hidden="true"></span>

      {#if fileOperations.renamingPath === entry.path}
        <!-- svelte-ignore a11y_autofocus -->
        <input
          class="file-row__rename"
          type="text"
          autofocus
          bind:value={renameDraft}
          onfocus={(event) => event.currentTarget.select()}
          onclick={(event) => event.stopPropagation()}
          onpointerdown={(event) => event.stopPropagation()}
          onblur={() => commitRename(entry.path)}
          onkeydown={(event) => {
            if (event.key === 'Enter') commitRename(entry.path)
            else if (event.key === 'Escape') fileOperations.cancelRenaming()
          }}
        />
      {:else}
        <span class="file-row__name">{entry.name}</span>
      {/if}

      <span class="file-row__kind">{entry.entryType === 'directory' ? 'Folder' : 'File'}</span>
    </div>
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
