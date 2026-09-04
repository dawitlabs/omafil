<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { onMount } from 'svelte'
  import ClockIcon from '@fluentui/svg-icons/icons/clock_20_regular.svg?no-inline'
  import FolderIcon from '@fluentui/svg-icons/icons/folder_20_regular.svg?no-inline'
  import HardDriveFilledIcon from '@fluentui/svg-icons/icons/hard_drive_20_filled.svg?no-inline'
  import HardDriveIcon from '@fluentui/svg-icons/icons/hard_drive_20_regular.svg?no-inline'
  import PinIcon from '@fluentui/svg-icons/icons/pin_20_regular.svg?no-inline'
  import UsbStickFilledIcon from '@fluentui/svg-icons/icons/usb_stick_20_filled.svg?no-inline'
  import { fileIcon, folderIcon } from '../../fileIcons'
  import { navigation } from '../../navigation.svelte'
  import type { DriveInfo, RecentFile } from '../../navigation.svelte'

  const pinnedItems = [
    { label: 'Desktop', type: 'desktop', tag: '', location: 'desktop' as const },
    { label: 'Concepts', type: 'folder', tag: 'red', location: null },
    { label: 'Code', type: 'folder', tag: 'yellow', location: null },
  ]

  let drives = $state<DriveInfo[]>([])
  let isLoadingDrives = $state(true)
  let drivesError = $state<string | null>(null)
  let recentFiles = $state<RecentFile[]>([])
  let isLoadingRecentFiles = $state(true)
  let recentFilesError = $state<string | null>(null)

  function readableError(error: unknown, fallback: string): string {
    if (typeof error === 'object' && error !== null && 'message' in error && typeof error.message === 'string') {
      return error.message
    }

    return fallback
  }

  function formatBytes(bytes: number): string {
    const units = ['B', 'KB', 'MB', 'GB', 'TB']
    let unitIndex = 0
    let value = bytes

    while (value >= 1024 && unitIndex < units.length - 1) {
      value /= 1024
      unitIndex += 1
    }

    return `${value >= 10 || unitIndex === 0 ? Math.round(value) : value.toFixed(1)} ${units[unitIndex]}`
  }

  function usedPercentage(drive: DriveInfo): number {
    if (drive.totalBytes === 0) return 0

    return Math.min(100, Math.max(0, ((drive.totalBytes - drive.availableBytes) / drive.totalBytes) * 100))
  }

  function openPinned(item: (typeof pinnedItems)[number]) {
    if (item.location) {
      void navigation.openLocation(item.location)
      return
    }

    navigation.openPlaceholder(item.label)
  }

  async function loadDrives() {
    isLoadingDrives = true
    drivesError = null

    try {
      drives = await invoke<DriveInfo[]>('list_drives')
    } catch (error) {
      drivesError = readableError(error, 'Unable to discover mounted drives.')
    } finally {
      isLoadingDrives = false
    }
  }

  async function loadRecentFiles() {
    isLoadingRecentFiles = true
    recentFilesError = null

    try {
      recentFiles = await invoke<RecentFile[]>('list_recent_files')
    } catch (error) {
      recentFiles = []
      recentFilesError = readableError(error, 'Unable to read recent files.')
    } finally {
      isLoadingRecentFiles = false
    }
  }

  onMount(() => {
    void loadDrives()
    void loadRecentFiles()
  })
</script>

<div class="home-screen">
  <div class="home-view">
    {#if navigation.view.kind === 'home'}
      <section class="home-view__section" aria-labelledby="pinned-heading">
        <h1 id="pinned-heading" class="home-view__heading">
          <span class="masked-icon home-view__heading-icon" style="--icon: url({PinIcon})" aria-hidden="true"></span><span>Pinned</span>
        </h1>
        <div class="home-view__pinned-grid">
          {#each pinnedItems as item (item.label)}
            <button class="home-view__pinned-item" type="button" onclick={() => openPinned(item)}>
              <span class="home-view__folder-art home-view__folder-art--{item.type}" aria-hidden="true">
                {#if item.type === 'desktop'}<span class="home-view__desktop-dots"></span>{/if}
              </span>
              <span class="home-view__pinned-label">
                {#if item.tag}<span class="home-view__tag-dot home-view__tag-dot--{item.tag}" aria-hidden="true"></span>{/if}<span>{item.label}</span>
              </span>
            </button>
          {/each}
        </div>
      </section>

      <section class="home-view__section" aria-labelledby="drives-heading">
        <h2 id="drives-heading" class="home-view__heading">
          <span class="masked-icon home-view__heading-icon" style="--icon: url({HardDriveIcon})" aria-hidden="true"></span><span>Drives</span>
        </h2>
        {#if isLoadingDrives}
          <p class="home-view__state">Discovering mounted drives…</p>
        {:else if drivesError}
          <div class="home-view__error" role="alert">
            <p>{drivesError}</p>
            <button class="home-view__retry" type="button" onclick={loadDrives}>Try again</button>
          </div>
        {:else if drives.length === 0}
          <p class="home-view__state">No mounted drives are available.</p>
        {:else}
          <div class="home-view__drive-grid">
            {#each drives as drive (drive.mountPoint)}
              <button class="home-view__drive-card" type="button" onclick={() => navigation.open(drive.path)} aria-label={`Open ${drive.name || drive.mountPoint}`}>
                <span
                  class="masked-icon home-view__drive-icon"
                  class:home-view__drive-icon--removable={drive.isRemovable}
                  style="--icon: url({drive.isRemovable ? UsbStickFilledIcon : HardDriveFilledIcon})"
                  aria-hidden="true"
                ></span>
                <span class="home-view__drive-content">
                  <strong>{drive.name || drive.mountPoint}</strong>
                  <span>{formatBytes(drive.availableBytes)} free of {formatBytes(drive.totalBytes)}</span>
                  <span class="home-view__progress" aria-label={`${Math.round(usedPercentage(drive))} percent used`}>
                    <span class="home-view__progress-fill" style:width={`${usedPercentage(drive)}%`}></span>
                  </span>
                </span>
              </button>
            {/each}
          </div>
        {/if}
      </section>

      <section class="home-view__section" aria-labelledby="recent-heading">
        <h2 id="recent-heading" class="home-view__heading">
          <span class="masked-icon home-view__heading-icon" style="--icon: url({ClockIcon})" aria-hidden="true"></span><span>Recently used files</span>
        </h2>
        {#if isLoadingRecentFiles}
          <p class="home-view__state">Loading recently used files…</p>
        {:else if recentFilesError}
          <div class="home-view__error" role="alert">
            <p>{recentFilesError}</p>
            <button class="home-view__retry" type="button" onclick={loadRecentFiles}>Try again</button>
          </div>
        {:else if recentFiles.length === 0}
          <p class="home-view__state">No recent files are available from this desktop yet.</p>
        {:else}
          <div class="home-view__recent-list">
            {#each recentFiles as file (file.path)}
              <button class="home-view__recent-row" type="button" onclick={() => navigation.openExternally(file.path)}>
                <span class="masked-icon home-view__recent-icon" style="--icon: url({fileIcon(file.name).icon}); color: {fileIcon(file.name).tone}" aria-hidden="true"></span>
                <span>{file.name}</span>
                <span class="home-view__recent-location">{file.parentDirectory}</span>
              </button>
            {/each}
          </div>
        {/if}
      </section>
    {:else}
      <section class="home-view__section" aria-labelledby="folder-heading">
        <h1 id="folder-heading" class="home-view__heading">
          <span class="masked-icon home-view__heading-icon" style="--icon: url({FolderIcon})" aria-hidden="true"></span><span>{navigation.label}</span>
        </h1>

        {#if navigation.view.kind === 'placeholder'}
          <p class="home-view__state">This location is not connected yet.</p>
        {:else if navigation.isLoading}
          <p class="home-view__state">Loading {navigation.label}…</p>
        {:else if navigation.error}
          <div class="home-view__error" role="alert">
            <p>{navigation.error}</p>
            <button class="home-view__retry" type="button" onclick={() => navigation.reload()}>Try again</button>
          </div>
        {:else if navigation.listing?.entries.length === 0}
          <p class="home-view__state">This folder is empty.</p>
        {:else if navigation.listing}
          <div class="home-view__recent-list">
            {#each navigation.listing.entries as entry (entry.path)}
              <button class="home-view__recent-row" type="button" onclick={() => navigation.openEntry(entry)}>
                <span
                  class="masked-icon home-view__recent-icon"
                  style="--icon: url({(entry.entryType === 'directory' ? folderIcon : fileIcon(entry.name)).icon}); color: {(entry.entryType === 'directory' ? folderIcon : fileIcon(entry.name)).tone}"
                  aria-hidden="true"
                ></span>
                <span>{entry.name}</span>
                <span class="home-view__recent-location">{entry.entryType === 'directory' ? 'Folder' : 'File'}</span>
              </button>
            {/each}
          </div>
        {/if}
      </section>
    {/if}
  </div>

  <footer class="home-status" aria-label="Folder status">
    <span>{navigation.label}</span>
    <span>
      {#if navigation.view.kind === 'home'}
        {recentFiles.length} items
      {:else if navigation.isLoading}
        Loading…
      {:else}
        {navigation.listing?.entries.length ?? 0} items
      {/if}
    </span>
  </footer>
</div>
