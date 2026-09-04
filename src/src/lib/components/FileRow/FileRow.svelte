<script lang="ts">
  import { fileIcon, folderIcon } from '../../fileIcons'
  import { fileOperations } from '../../fileOperations.svelte'
  import { formatBytes, formatModified, parentFolder, typeLabel } from '../../format'
  import type { DirectoryEntry } from '../../navigation.svelte'

  let {
    entry,
    index,
    isActive,
    showFolder,
    onactivate,
    onopen,
    onmenu,
  }: {
    entry: DirectoryEntry
    index: number
    isActive: boolean
    showFolder: boolean
    onactivate: (event: MouseEvent) => void
    onopen: () => void
    onmenu: (event: MouseEvent) => void
  } = $props()

  const isDirectory = $derived(entry.entryType === 'directory')
  const icon = $derived(isDirectory ? folderIcon : fileIcon(entry.name))
  const isRenaming = $derived(fileOperations.renamingPath === entry.path)
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
>
  <span class="masked-icon file-row__icon" style="--icon: url({icon.icon}); color: {icon.tone}" aria-hidden="true"></span>

  {#if isRenaming}
    <!-- svelte-ignore a11y_autofocus -->
    <input
      class="file-row__rename"
      type="text"
      autofocus
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
    <span class="file-row__name">{entry.name}</span>
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
