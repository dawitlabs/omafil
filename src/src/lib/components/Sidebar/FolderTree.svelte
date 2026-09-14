<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import ChevronRightIcon from '@fluentui/svg-icons/icons/chevron_right_20_regular.svg?no-inline'
  import FolderIcon from '@fluentui/svg-icons/icons/folder_20_regular.svg?no-inline'
  import FolderTree from './FolderTree.svelte'
  import { appState } from '../../appState.svelte'
  import { fileOperations } from '../../fileOperations.svelte'
  import { folderMenuItems } from '../../folderMenu'
  import type { ContextMenuItem } from '../ContextMenu/ContextMenu.svelte'
  import type { PathCrumb } from '../../navigation.svelte'
  import { tabs } from '../../tabs.svelte'

  let { path, depth = 1, onmenu }: { path: string; depth?: number; onmenu: (event: MouseEvent, items: ContextMenuItem[]) => void } = $props()

  let folders = $state<PathCrumb[]>([])
  let isLoading = $state(true)
  let failed = $state(false)
  let expanded = $state<string[]>([])
  let renamingPath = $state<string | null>(null)
  let renameDraft = $state('')

  const active = $derived(tabs.active)

  function reload() {
    folders = []
    isLoading = true
    void load(path)
  }

  function startRename(folder: PathCrumb) {
    renamingPath = folder.path
    renameDraft = folder.name
  }

  async function commitRename(folder: PathCrumb) {
    const name = renameDraft
    renamingPath = null

    if (await fileOperations.renamePath(folder.path, name)) reload()
  }

  function load(requested: string) {
    failed = false

    return invoke<PathCrumb[]>('list_subdirectories', { path: requested, showHidden: appState.settings.showHidden })
      .then((found) => {
        if (requested === path) folders = found
      })
      .catch(() => {
        if (requested === path) failed = true
      })
      .finally(() => {
        if (requested === path) isLoading = false
      })
  }

  $effect(() => {
    isLoading = true
    void load(path)
  })

  function toggle(folderPath: string) {
    expanded = expanded.includes(folderPath) ? expanded.filter((open) => open !== folderPath) : [...expanded, folderPath]
  }
</script>

{#if isLoading}
  <span class="file-sidebar__empty-state" style="--depth: {depth}">Loading…</span>
{:else if failed}
  <span class="file-sidebar__empty-state file-sidebar__empty-state--error" style="--depth: {depth}">Unable to read this folder</span>
{:else}
  {#each folders as folder (folder.path)}
    {@const isOpen = expanded.includes(folder.path)}
    <div class="file-sidebar__tree-row" style="--depth: {depth}">
      <button
        class="file-sidebar__twisty"
        class:file-sidebar__twisty--open={isOpen}
        type="button"
        aria-expanded={isOpen}
        aria-label={isOpen ? `Collapse ${folder.name}` : `Expand ${folder.name}`}
        onclick={() => toggle(folder.path)}
      >
        <span class="masked-icon" style="--icon: url({ChevronRightIcon})" aria-hidden="true"></span>
      </button>
      {#if renamingPath === folder.path}
        <div class="file-sidebar__item file-sidebar__item--tree file-sidebar__item--editing">
          <span class="masked-icon file-sidebar__icon" style="--icon: url({FolderIcon})" aria-hidden="true"></span>
          <!-- svelte-ignore a11y_autofocus -->
          <input
            class="file-sidebar__tag-input"
            type="text"
            autofocus
            aria-label="Folder name"
            bind:value={renameDraft}
            onblur={() => commitRename(folder)}
            onkeydown={(event) => {
              if (event.key === 'Enter') event.currentTarget.blur()
              else if (event.key === 'Escape') renamingPath = null
            }}
          />
        </div>
      {:else}
        <button
          class="file-sidebar__item file-sidebar__item--tree"
          class:file-sidebar__item--active={active.isCurrentPath(folder.path)}
          type="button"
          aria-current={active.isCurrentPath(folder.path) ? 'page' : undefined}
          onclick={() => active.open(folder.path)}
          oncontextmenu={(event) => {
            event.preventDefault()
            onmenu(event, folderMenuItems(folder.path, { onRename: () => startRename(folder), onChanged: reload }))
          }}
        >
          <span class="masked-icon file-sidebar__icon" style="--icon: url({FolderIcon})" aria-hidden="true"></span>
          <span>{folder.name}</span>
        </button>
      {/if}
    </div>
    {#if isOpen}
      <FolderTree path={folder.path} depth={depth + 1} {onmenu} />
    {/if}
  {:else}
    <span class="file-sidebar__empty-state" style="--depth: {depth}">No folders</span>
  {/each}
{/if}
