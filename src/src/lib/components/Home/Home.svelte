<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { onMount } from 'svelte'
  import ClockIcon from '@fluentui/svg-icons/icons/clock_20_regular.svg?no-inline'
  import FolderIcon from '@fluentui/svg-icons/icons/folder_20_regular.svg?no-inline'
  import HardDriveFilledIcon from '@fluentui/svg-icons/icons/hard_drive_20_filled.svg?no-inline'
  import HardDriveIcon from '@fluentui/svg-icons/icons/hard_drive_20_regular.svg?no-inline'
  import PinIcon from '@fluentui/svg-icons/icons/pin_20_regular.svg?no-inline'
  import UsbStickFilledIcon from '@fluentui/svg-icons/icons/usb_stick_20_filled.svg?no-inline'
  import FileList from '../FileList/FileList.svelte'
  import { appState } from '../../appState.svelte'
  import { fileIcon } from '../../fileIcons'
  import { fileOperations } from '../../fileOperations.svelte'
  import { formatBytes, formatCount } from '../../format'
  import { tabs } from '../../tabs.svelte'
  import type { DriveInfo, RecentFile } from '../../navigation.svelte'

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

  function usedPercentage(drive: DriveInfo): number {
    if (drive.totalBytes === 0) return 0

    return Math.min(100, Math.max(0, ((drive.totalBytes - drive.availableBytes) / drive.totalBytes) * 100))
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
    {#if tabs.active.view.kind === 'home'}
      <section class="home-view__section" aria-labelledby="pinned-heading">
        <h1 id="pinned-heading" class="home-view__heading">
          <span class="masked-icon home-view__heading-icon" style="--icon: url({PinIcon})" aria-hidden="true"></span><span>Pinned</span>
        </h1>
        <div class="home-view__pinned-grid">
          {#each appState.pins as pin (pin.path)}
            <button class="home-view__pinned-item" type="button" onclick={() => tabs.active.open(pin.path)}>
              <span class="home-view__folder-art" aria-hidden="true"></span>
              <span class="home-view__pinned-label">
                {#each appState.tagsFor(pin.path) as tag (tag.id)}
                  <span class="home-view__tag-dot" style="background: {tag.color}" aria-hidden="true"></span>
                {/each}
                <span>{pin.label}</span>
              </span>
            </button>
          {:else}
            <p class="home-view__state">Right-click a folder and choose Pin to sidebar to see it here.</p>
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
              <button class="home-view__drive-card" type="button" onclick={() => tabs.active.open(drive.path)} aria-label={`Open ${drive.name || drive.mountPoint}`}>
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
              <button class="home-view__recent-row" type="button" onclick={() => tabs.active.openExternally(file.path)}>
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
          <span class="masked-icon home-view__heading-icon" style="--icon: url({FolderIcon})" aria-hidden="true"></span><span>{tabs.active.label}</span>
        </h1>

        {#if tabs.active.view.kind === 'placeholder'}
          <p class="home-view__state">This location is not connected yet.</p>
        {:else if tabs.active.isLoading}
          <p class="home-view__state">Loading {tabs.active.label}…</p>
        {:else if tabs.active.error}
          <div class="home-view__error" role="alert">
            <p>{tabs.active.error}</p>
            <button class="home-view__retry" type="button" onclick={() => tabs.active.reload()}>Try again</button>
          </div>
        {:else}
          <FileList />
        {/if}
      </section>
    {/if}
  </div>

  <footer class="home-status" aria-label="Folder status">
    <span>{tabs.active.label}</span>
    <span>
      {#if tabs.active.view.kind === 'home'}
        {recentFiles.length} items
      {:else if tabs.active.isLoading}
        Loading…
      {:else}
        {formatCount(tabs.active.total)} items{fileOperations.selectedPaths.length > 0
          ? ` · ${fileOperations.selectedPaths.length} selected`
          : ''}
      {/if}
    </span>
  </footer>
</div>
