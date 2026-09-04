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
  import { knownLocations, navigation } from '../../navigation.svelte'
  import type { DriveInfo, KnownLocation } from '../../navigation.svelte'

  type LocationItem = {
    location: KnownLocation
    label: string
    icon: string
  }

  const pinnedItems: LocationItem[] = [{ location: 'desktop', label: 'Desktop', icon: DesktopIcon }]

  const pinnedPlaceholders = [
    { label: 'Concepts', icon: FolderIcon },
    { label: 'Code', icon: FolderIcon },
  ]

  const fileItems: LocationItem[] = [
    { location: 'downloads', label: 'Downloads', icon: DownloadIcon },
    { location: 'documents', label: 'Documents', icon: DocumentIcon },
    { location: 'pictures', label: 'Pictures', icon: ImageIcon },
    { location: 'videos', label: 'Videos', icon: VideoIcon },
    { location: 'music', label: 'Music', icon: MusicIcon },
  ]

  const tags = [
    { label: 'Design', color: 'red' },
    { label: 'Dev', color: 'yellow' },
    { label: 'School', color: 'blue' },
  ]

  let drives = $state<DriveInfo[]>([])
  let drivesError = $state(false)
  let locationPaths = $state<Partial<Record<KnownLocation, string>>>({})

  function isPlaceholderActive(label: string): boolean {
    const view = navigation.view

    return view.kind === 'placeholder' && view.label === label
  }

  function isLocationActive(location: KnownLocation): boolean {
    const path = locationPaths[location]

    return path !== undefined && navigation.isCurrentPath(path)
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
      class:file-sidebar__item--active={navigation.view.kind === 'home'}
      type="button"
      aria-current={navigation.view.kind === 'home' ? 'page' : undefined}
      onclick={() => navigation.goHome()}
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
      {#each pinnedItems as item (item.location)}
        <button
          class="file-sidebar__item"
          class:file-sidebar__item--active={isLocationActive(item.location)}
          type="button"
          aria-current={isLocationActive(item.location) ? 'page' : undefined}
          onclick={() => navigation.openLocation(item.location)}
        >
          <span class="masked-icon file-sidebar__icon" style="--icon: url({item.icon})" aria-hidden="true"></span>
          <span>{item.label}</span>
        </button>
      {/each}
      {#each pinnedPlaceholders as item (item.label)}
        <button
          class="file-sidebar__item"
          class:file-sidebar__item--active={isPlaceholderActive(item.label)}
          type="button"
          aria-current={isPlaceholderActive(item.label) ? 'page' : undefined}
          onclick={() => navigation.openPlaceholder(item.label)}
        >
          <span class="masked-icon file-sidebar__icon" style="--icon: url({item.icon})" aria-hidden="true"></span>
          <span>{item.label}</span>
        </button>
      {/each}
    </nav>
  </section>

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
          onclick={() => navigation.openLocation(item.location)}
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
            class:file-sidebar__item--active={navigation.isCurrentPath(drive.path)}
            type="button"
            aria-current={navigation.isCurrentPath(drive.path) ? 'page' : undefined}
            onclick={() => navigation.open(drive.path)}
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
      {#each tags as tag (tag.label)}
        <button
          class="file-sidebar__item"
          class:file-sidebar__item--active={isPlaceholderActive(tag.label)}
          type="button"
          aria-current={isPlaceholderActive(tag.label) ? 'page' : undefined}
          onclick={() => navigation.openPlaceholder(tag.label)}
        >
          <span class="file-sidebar__tag-dot file-sidebar__tag-dot--{tag.color}" aria-hidden="true"></span>
          <span>{tag.label}</span>
        </button>
      {/each}
      <button
        class="file-sidebar__item"
        class:file-sidebar__item--active={isPlaceholderActive('Create new tag')}
        type="button"
        aria-current={isPlaceholderActive('Create new tag') ? 'page' : undefined}
        onclick={() => navigation.openPlaceholder('Create new tag')}
      >
        <span class="masked-icon file-sidebar__icon" style="--icon: url({AddIcon})" aria-hidden="true"></span>
        <span>Create new tag</span>
      </button>
    </nav>
  </section>
</aside>
