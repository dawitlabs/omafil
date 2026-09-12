<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { onMount } from 'svelte'
  import AddIcon from '@fluentui/svg-icons/icons/add_20_regular.svg?no-inline'
  import DesktopIcon from '@fluentui/svg-icons/icons/desktop_20_regular.svg?no-inline'
  import DocumentIcon from '@fluentui/svg-icons/icons/document_20_regular.svg?no-inline'
  import DownloadIcon from '@fluentui/svg-icons/icons/arrow_download_20_regular.svg?no-inline'
  import FolderFilledIcon from '@fluentui/svg-icons/icons/folder_20_filled.svg?no-inline'
  import FolderIcon from '@fluentui/svg-icons/icons/folder_20_regular.svg?no-inline'
  import HardDriveIcon from '@fluentui/svg-icons/icons/hard_drive_20_regular.svg?no-inline'
  import HomeFilledIcon from '@fluentui/svg-icons/icons/home_20_filled.svg?no-inline'
  import ImageIcon from '@fluentui/svg-icons/icons/image_20_regular.svg?no-inline'
  import MusicIcon from '@fluentui/svg-icons/icons/music_note_2_20_regular.svg?no-inline'
  import PinIcon from '@fluentui/svg-icons/icons/pin_20_regular.svg?no-inline'
  import TagIcon from '@fluentui/svg-icons/icons/tag_20_regular.svg?no-inline'
  import UsbStickIcon from '@fluentui/svg-icons/icons/usb_stick_20_regular.svg?no-inline'
  import VideoIcon from '@fluentui/svg-icons/icons/video_20_regular.svg?no-inline'
  import DeleteIcon from '@fluentui/svg-icons/icons/delete_20_regular.svg?no-inline'
  import ContextMenu from '../ContextMenu/ContextMenu.svelte'
  import type { ContextMenuItem } from '../ContextMenu/ContextMenu.svelte'
  import { appState } from '../../appState.svelte'
  import { fileOperations } from '../../fileOperations.svelte'
  import { knownLocations } from '../../navigation.svelte'
  import type { DriveInfo, KnownLocation } from '../../navigation.svelte'
  import { tabs } from '../../tabs.svelte'

  type LocationItem = {
    location: KnownLocation
    label: string
    icon: string
  }

  const fileItems: LocationItem[] = [
    { location: 'desktop', label: 'Desktop', icon: DesktopIcon },
    { location: 'downloads', label: 'Downloads', icon: DownloadIcon },
    { location: 'documents', label: 'Documents', icon: DocumentIcon },
    { location: 'pictures', label: 'Pictures', icon: ImageIcon },
    { location: 'videos', label: 'Videos', icon: VideoIcon },
    { location: 'music', label: 'Music', icon: MusicIcon },
  ]

  let drives = $state<DriveInfo[]>([])
  let drivesError = $state(false)
  let locationPaths = $state<Partial<Record<KnownLocation, string>>>({})
  let newTagLabel = $state<string | null>(null)
  let menu = $state<{ x: number; y: number; items: ContextMenuItem[] } | null>(null)

  const active = $derived(tabs.active)

  function isLocationActive(location: KnownLocation): boolean {
    const path = locationPaths[location]

    return path !== undefined && active.isCurrentPath(path)
  }

  function isTagActive(id: string): boolean {
    const view = active.view

    return view.kind === 'tag' && view.id === id
  }

  function openMenu(event: MouseEvent, items: ContextMenuItem[]) {
    event.preventDefault()
    menu = { x: event.clientX, y: event.clientY, items }
  }

  function commitNewTag() {
    const label = (newTagLabel ?? '').trim()

    if (label) appState.createTag(label)

    newTagLabel = null
  }

  async function loadDrives() {
    drivesError = false

    try {
      drives = await invoke<DriveInfo[]>('list_drives')
    } catch {
      drives = []
      drivesError = true
    }
  }

  async function loadLocationPaths() {
    const resolved = await Promise.all(
      knownLocations.map(async (location) => {
        try {
          return [location, await invoke<string>('resolve_location', { location })] as const
        } catch {
          return [location, undefined] as const
        }
      }),
    )

    locationPaths = Object.fromEntries(resolved.filter(([, path]) => path !== undefined))
  }

  onMount(() => {
    void loadDrives()
    void loadLocationPaths()
  })
</script>

<aside class="file-sidebar" aria-label="File navigation">
  <div class="file-sidebar__brand">
    <span class="masked-icon file-sidebar__icon" style="--icon: url({FolderFilledIcon})" aria-hidden="true"></span>
    <span>omafiles</span>
  </div>

  <nav class="file-sidebar__primary-nav" aria-label="Primary navigation">
    <button
      class="file-sidebar__item"
      class:file-sidebar__item--active={active.view.kind === 'home'}
      type="button"
      aria-current={active.view.kind === 'home' ? 'page' : undefined}
      onclick={() => active.goHome()}
    >
      <span class="masked-icon file-sidebar__icon" style="--icon: url({HomeFilledIcon})" aria-hidden="true"></span>
      <span>Home</span>
    </button>
  </nav>

  <section class="file-sidebar__section" aria-labelledby="sidebar-pinned-heading">
    <h2 id="sidebar-pinned-heading" class="file-sidebar__section-heading">
      <span class="masked-icon file-sidebar__icon" style="--icon: url({PinIcon})" aria-hidden="true"></span>
      <span>Pinned</span>
    </h2>
    <nav class="file-sidebar__nav" aria-label="Pinned locations">
      {#each appState.pins as pin (pin.path)}
        <button
          class="file-sidebar__item"
          class:file-sidebar__item--active={active.isCurrentPath(pin.path)}
          type="button"
          aria-current={active.isCurrentPath(pin.path) ? 'page' : undefined}
          onclick={() => active.open(pin.path)}
          oncontextmenu={(event) =>
            openMenu(event, [{ kind: 'action', label: 'Unpin from sidebar', onSelect: () => appState.togglePin(pin.path) }])}
        >
          <span class="masked-icon file-sidebar__icon" style="--icon: url({FolderIcon})" aria-hidden="true"></span>
          <span>{pin.label}</span>
        </button>
      {:else}
        <span class="file-sidebar__empty-state">Right-click a folder to pin it</span>
      {/each}
    </nav>
  </section>

  <nav class="file-sidebar__primary-nav" aria-label="Recovery">
    <button class="file-sidebar__item" class:file-sidebar__item--active={active.view.kind === 'recycle'} type="button" aria-current={active.view.kind === 'recycle' ? 'page' : undefined} onclick={() => active.openRecycleBin()}>
      <span class="masked-icon file-sidebar__icon" style="--icon: url({DeleteIcon})" aria-hidden="true"></span>
      <span>Recycle Bin</span>
    </button>
  </nav>

  <section class="file-sidebar__section" aria-labelledby="sidebar-files-heading">
    <h2 id="sidebar-files-heading" class="file-sidebar__section-heading">
      <span class="masked-icon file-sidebar__icon" style="--icon: url({FolderIcon})" aria-hidden="true"></span>
      <span>Your Files</span>
    </h2>
    <nav class="file-sidebar__nav" aria-label="Your files">
      {#each fileItems as item (item.location)}
        <button
          class="file-sidebar__item"
          class:file-sidebar__item--active={isLocationActive(item.location)}
          type="button"
          aria-current={isLocationActive(item.location) ? 'page' : undefined}
          onclick={() => active.openLocation(item.location)}
          oncontextmenu={(event) => openMenu(event, [{ kind: 'action', label: 'Open', onSelect: () => active.openLocation(item.location) }, { kind: 'action', label: 'Open in new tab', onSelect: async () => { const path = await invoke<string>('resolve_location', { location: item.location }); tabs.open(); tabs.active.open(path) } }, { kind: 'action', label: 'Open externally', onSelect: async () => active.openExternally(await invoke<string>('resolve_location', { location: item.location })) }, { kind: 'separator' }, { kind: 'action', label: 'Cut', shortcut: 'Ctrl+X', onSelect: async () => fileOperations.cutPaths([await invoke<string>('resolve_location', { location: item.location })]) }, { kind: 'action', label: 'Copy', shortcut: 'Ctrl+C', onSelect: async () => fileOperations.copyPaths([await invoke<string>('resolve_location', { location: item.location })]) }, { kind: 'action', label: 'Paste into folder', shortcut: 'Ctrl+V', disabled: !fileOperations.canPaste, onSelect: async () => fileOperations.pasteTo(await invoke<string>('resolve_location', { location: item.location })) }, { kind: 'separator' }, { kind: 'action', label: 'Pin to sidebar', onSelect: async () => appState.togglePin(await invoke<string>('resolve_location', { location: item.location })) }, { kind: 'action', label: 'Copy path', onSelect: async () => navigator.clipboard.writeText(await invoke<string>('resolve_location', { location: item.location })) }, { kind: 'action', label: 'Properties', onSelect: async () => fileOperations.showProperties(await invoke<string>('resolve_location', { location: item.location })) }, { kind: 'action', label: 'Refresh', onSelect: () => active.reload() }])}
        >
          <span class="masked-icon file-sidebar__icon" style="--icon: url({item.icon})" aria-hidden="true"></span>
          <span>{item.label}</span>
        </button>
      {/each}
    </nav>
  </section>

  <section class="file-sidebar__section" aria-labelledby="sidebar-drives-heading">
    <h2 id="sidebar-drives-heading" class="file-sidebar__section-heading">
      <span class="masked-icon file-sidebar__icon" style="--icon: url({HardDriveIcon})" aria-hidden="true"></span>
      <span>Drives</span>
    </h2>
    <nav class="file-sidebar__nav" aria-label="Drives">
      {#if drivesError}
        <span class="file-sidebar__empty-state">Unable to load drives</span>
      {:else if drives.length === 0}
        <span class="file-sidebar__empty-state">No mounted drives</span>
      {:else}
        {#each drives as drive (drive.mountPoint)}
          <button
            class="file-sidebar__item"
            class:file-sidebar__item--active={active.isCurrentPath(drive.path)}
            type="button"
            aria-current={active.isCurrentPath(drive.path) ? 'page' : undefined}
            onclick={() => active.open(drive.path)}
            oncontextmenu={(event) => openMenu(event, [{ kind: 'action', label: 'Open', onSelect: () => active.open(drive.path) }, { kind: 'action', label: 'Open in new tab', onSelect: () => { tabs.open(); tabs.active.open(drive.path) } }, { kind: 'separator' }, { kind: 'action', label: appState.isPinned(drive.path) ? 'Unpin from sidebar' : 'Pin to sidebar', onSelect: () => appState.togglePin(drive.path) }, { kind: 'action', label: 'Copy path', onSelect: () => navigator.clipboard.writeText(drive.path) }])}
          >
            <span class="masked-icon file-sidebar__icon" style="--icon: url({drive.isRemovable ? UsbStickIcon : HardDriveIcon})" aria-hidden="true"></span>
            <span>{drive.name || drive.mountPoint}</span>
          </button>
        {/each}
      {/if}
    </nav>
  </section>

  <section class="file-sidebar__section" aria-labelledby="sidebar-tags-heading">
    <h2 id="sidebar-tags-heading" class="file-sidebar__section-heading">
      <span class="masked-icon file-sidebar__icon" style="--icon: url({TagIcon})" aria-hidden="true"></span>
      <span>Tags</span>
    </h2>
    <nav class="file-sidebar__nav" aria-label="Tags">
      {#each appState.tags as tag (tag.id)}
        <button
          class="file-sidebar__item"
          class:file-sidebar__item--active={isTagActive(tag.id)}
          type="button"
          aria-current={isTagActive(tag.id) ? 'page' : undefined}
          onclick={() => active.openTag(tag.id, tag.label)}
          oncontextmenu={(event) =>
            openMenu(event, [{ kind: 'action', label: `Delete “${tag.label}”`, onSelect: () => appState.removeTag(tag.id) }])}
        >
          <span class="file-sidebar__tag-dot" style="background: {tag.color}" aria-hidden="true"></span>
          <span>{tag.label}</span>
        </button>
      {/each}

      {#if newTagLabel === null}
        <button class="file-sidebar__item" type="button" onclick={() => (newTagLabel = '')}>
          <span class="masked-icon file-sidebar__icon" style="--icon: url({AddIcon})" aria-hidden="true"></span>
          <span>Create new tag</span>
        </button>
      {:else}
        <div class="file-sidebar__item file-sidebar__item--editing">
          <span class="masked-icon file-sidebar__icon" style="--icon: url({TagIcon})" aria-hidden="true"></span>
          <!-- svelte-ignore a11y_autofocus -->
          <input
            class="file-sidebar__tag-input"
            type="text"
            autofocus
            aria-label="New tag name"
            placeholder="Tag name"
            bind:value={newTagLabel}
            onblur={commitNewTag}
            onkeydown={(event) => {
              if (event.key === 'Enter') commitNewTag()
              else if (event.key === 'Escape') newTagLabel = null
            }}
          />
        </div>
      {/if}
    </nav>
  </section>
</aside>

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} items={menu.items} onclose={() => (menu = null)} />
{/if}
