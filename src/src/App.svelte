<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { onMount } from 'svelte'
  import { getCurrentWindow } from '@tauri-apps/api/window'
  import OperationQueue from './lib/components/OperationQueue/OperationQueue.svelte'
  import Header from './lib/components/Header/Header.svelte'
  import FileInspector from './lib/components/FileInspector/FileInspector.svelte'
  import BulkRename from './lib/components/BulkRename/BulkRename.svelte'
  import OpenWith from './lib/components/OpenWith/OpenWith.svelte'
  import Home from './lib/components/Home/Home.svelte'
  import Sidebar from './lib/components/Sidebar/Sidebar.svelte'
  import { appState } from './lib/appState.svelte'
  import { driveStore } from './lib/drives.svelte'
  import { fileOperations } from './lib/fileOperations.svelte'
  import { tabs } from './lib/tabs.svelte'

  let watchedPath = $state<string | null>(null)

  onMount(() => {
    void appState.load()
    let reloadTimer: ReturnType<typeof setTimeout> | null = null

    const stopListening = listen<string>('directory-changed', (event) => {
      if (event.payload !== tabs.active.directoryPath) return

      // Linux file managers and media players commonly produce a burst of
      // inotify events for one user action. Reload once after that burst,
      // rather than replacing the entire folder view for every event.
      if (reloadTimer) clearTimeout(reloadTimer)
      reloadTimer = setTimeout(() => {
        reloadTimer = null
        if (event.payload === tabs.active.directoryPath) tabs.active.reload()
      }, 450)
    })

    const stopOperationListening = listen<import('./lib/fileOperations.svelte').FileOperation>('file-operation', (event) => {
      fileOperations.receiveOperationUpdate(event.payload)
    })

    const stopThemeListening = listen('omarchy-theme-changed', () => {
      void appState.refreshOmarchyTheme()
    })

    const stopDriveListening = listen('drives-changed', () => {
      void driveStore.load()
    })

    const stopDropListening = getCurrentWindow().onDragDropEvent((event) => {
      if (event.payload.type === 'enter') fileOperations.externalDropPaths = event.payload.paths
      else if (event.payload.type === 'leave') fileOperations.externalDropPaths = null
      else if (event.payload.type === 'drop') {
        const paths = event.payload.paths
        fileOperations.externalDropPaths = null
        if (tabs.active.directoryPath) void fileOperations.importDroppedPaths(paths)
      }
    })

    return () => {
      void stopListening.then((stop) => stop())
      void stopOperationListening.then((stop) => stop())
      void stopThemeListening.then((stop) => stop())
      void stopDriveListening.then((stop) => stop())
      void stopDropListening.then((stop) => stop())
      if (reloadTimer) clearTimeout(reloadTimer)
      void invoke('unwatch_directory')
    }
  })

  // Only the visible folder is watched; switching tabs re-points the watcher.
  $effect(() => {
    const path = tabs.active.directoryPath

    // Reloading replaces the listing object even when its path is unchanged.
    // Do not tear down and recreate the native watcher for that same folder.
    if (path === watchedPath) return

    watchedPath = path || null
    if (path) void invoke('watch_directory', { path }).catch(() => {})
    else void invoke('unwatch_directory')
  })
</script>

<div class="app-shell">
  <Header />
  <Sidebar />

  <main class="app-main">
    <Home />
  </main>

  {#if fileOperations.externalDropPaths}
    <div class="app-drop-overlay" role="status">Copy {fileOperations.externalDropPaths.length} item{fileOperations.externalDropPaths.length === 1 ? '' : 's'} into this folder</div>
  {/if}

  {#if fileOperations.propertiesPath}
    <FileInspector path={fileOperations.propertiesPath} mode="properties" onclose={() => fileOperations.hideProperties()} />
  {/if}
  {#if fileOperations.openWithPath}
    <OpenWith path={fileOperations.openWithPath} onclose={() => fileOperations.hideOpenWith()} />
  {/if}
  {#if fileOperations.bulkRenamePaths}
    <BulkRename paths={fileOperations.bulkRenamePaths} onclose={() => fileOperations.hideBulkRename()} />
  {/if}

  <OperationQueue />
</div>
