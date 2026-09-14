<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import ChevronRightIcon from '@fluentui/svg-icons/icons/chevron_right_20_regular.svg?no-inline'
  import FolderIcon from '@fluentui/svg-icons/icons/folder_20_regular.svg?no-inline'
  import FolderTree from './FolderTree.svelte'
  import { appState } from '../../appState.svelte'
  import type { PathCrumb } from '../../navigation.svelte'
  import { tabs } from '../../tabs.svelte'

  let { path, depth = 1, oncontext }: { path: string; depth?: number; oncontext?: (event: MouseEvent, folder: PathCrumb) => void } = $props()

  let folders = $state<PathCrumb[]>([])
  let isLoading = $state(true)
  let failed = $state(false)
  let expanded = $state<string[]>([])

  const active = $derived(tabs.active)

  $effect(() => {
    const requested = path
    isLoading = true
    failed = false

    invoke<PathCrumb[]>('list_subdirectories', { path: requested, showHidden: appState.settings.showHidden })
      .then((found) => {
        if (requested === path) folders = found
      })
      .catch(() => {
        if (requested === path) failed = true
      })
      .finally(() => {
        if (requested === path) isLoading = false
      })
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
      <button
        class="file-sidebar__item file-sidebar__item--tree"
        class:file-sidebar__item--active={active.isCurrentPath(folder.path)}
        type="button"
        aria-current={active.isCurrentPath(folder.path) ? 'page' : undefined}
        onclick={() => active.open(folder.path)}
        oncontextmenu={(event) => oncontext?.(event, folder)}
      >
        <span class="masked-icon file-sidebar__icon" style="--icon: url({FolderIcon})" aria-hidden="true"></span>
        <span>{folder.name}</span>
      </button>
    </div>
    {#if isOpen}
      <FolderTree path={folder.path} depth={depth + 1} {oncontext} />
    {/if}
  {:else}
    <span class="file-sidebar__empty-state" style="--depth: {depth}">No folders</span>
  {/each}
{/if}
