<script lang="ts">
  import { tick } from 'svelte'
  import { appState } from '../../appState.svelte'
  import { convertFileSrc, invoke } from '@tauri-apps/api/core'
  import { fileIcon, folderIcon, folderThemeName } from '../../fileIcons'
  import { fileOperations } from '../../fileOperations.svelte'
  import { formatBytes, formatModified, parentFolder, typeLabel } from '../../format'
  import type { DirectoryEntry } from '../../navigation.svelte'

  let {
    entry,
    index,
    isActive,
    showFolder,
    previewMode,
    onactivate,
    onopen,
    onmenu,
    ondragstart,
    ondragover,
    ondrop,
  }: {
    entry: DirectoryEntry
    index: number
    isActive: boolean
    showFolder: boolean
    previewMode: boolean
    onactivate: (event: MouseEvent) => void
    onopen: () => void
    onmenu: (event: MouseEvent) => void
    ondragstart: (event: DragEvent) => void
    ondragover: (event: DragEvent) => void
    ondrop: (event: DragEvent) => void
  } = $props()

  const isDirectory = $derived(entry.entryType === 'directory')
  const icon = $derived(isDirectory ? folderIcon : fileIcon(entry.name))
  const themedIcon = $derived(appState.themeIconFor(isDirectory ? folderThemeName(entry.name) : icon.themeName))
  const isRenaming = $derived(fileOperations.renamingPath === entry.path)
  const imageFile = $derived(/\.(png|jpe?g|gif|webp|avif|bmp)$/i.test(entry.name))
  let pdfThumbnail = $state<string | null>(null)
  const thumbnailSource = $derived(imageFile ? convertFileSrc(entry.path) : pdfThumbnail)

  $effect(() => {
    pdfThumbnail = null
    if (!previewMode || !/\.pdf$/i.test(entry.name)) return
    const target = entry.path

    invoke<string>('pdf_preview', { path: target })
      .then((rendered) => {
        if (entry.path === target) pdfThumbnail = convertFileSrc(rendered)
      })
      .catch(() => undefined)
  })
  let renameInput = $state<HTMLInputElement | null>(null)

  $effect(() => {
    if (!isRenaming) return

    void tick().then(() => {
      renameInput?.focus()
      renameInput?.select()
    })
  })
</script>

<div
  class="file-row"
  class:file-row--selected={fileOperations.isSelected(entry.path)}
  class:file-row--cut={fileOperations.clipboardMode === 'cut' && fileOperations.clipboardPaths.includes(entry.path)}
  data-path={entry.path}
  data-index={index}
  role="option"
  aria-selected={fileOperations.isSelected(entry.path)}
  tabindex={isActive ? 0 : -1}
  onpointerdown={onactivate}
  ondblclick={onopen}
  oncontextmenu={(event) => {
    event.preventDefault()
    onmenu(event)
  }}
  draggable={!isRenaming}
  ondragstart={ondragstart}
  ondragover={ondragover}
  ondrop={ondrop}
>
  {#if previewMode && thumbnailSource}
    <img class="file-row__thumbnail" src={thumbnailSource} alt="" loading="lazy" decoding="async" />
  {:else if themedIcon}
    <img class="file-row__icon file-row__icon--themed" src={themedIcon} alt="" decoding="async" />
  {:else}
    <span class="masked-icon file-row__icon" style="--icon: url({icon.icon}); color: {icon.tone}" aria-hidden="true"></span>
  {/if}

  {#if isRenaming}
    <!-- svelte-ignore a11y_autofocus -->
    <input
      class="file-row__rename"
      type="text"
      autofocus
      bind:this={renameInput}
      aria-label="New name"
      bind:value={fileOperations.renameDraft}
      onfocus={(event) => event.currentTarget.select()}
      onpointerdown={(event) => event.stopPropagation()}
      onblur={() => fileOperations.rename(entry.path)}
      onkeydown={(event) => {
        if (event.key === 'Enter') fileOperations.rename(entry.path)
        else if (event.key === 'Escape') fileOperations.cancelRenaming()
      }}
    />
  {:else}
    <span class="file-row__name">
      <span class="file-row__label">{entry.name}</span>
      {#each appState.tagsFor(entry.path) as tag (tag.id)}
        <span class="file-row__tag" style="background: {tag.color}" title={tag.label}></span>
      {/each}
    </span>
  {/if}

  {#if showFolder}
    <span class="file-row__kind" title={parentFolder(entry.path)}>{parentFolder(entry.path)}</span>
    <span class="file-row__modified">{formatModified(entry.modified)}</span>
  {:else}
    <span class="file-row__modified">{formatModified(entry.modified)}</span>
    <span class="file-row__kind">{typeLabel(entry.name, isDirectory)}</span>
  {/if}
  <span class="file-row__size">{isDirectory ? '' : formatBytes(entry.size)}</span>
</div>
