<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { onMount } from 'svelte'
  import Header from './lib/components/Header/Header.svelte'
  import Home from './lib/components/Home/Home.svelte'
  import Sidebar from './lib/components/Sidebar/Sidebar.svelte'
  import { appState } from './lib/appState.svelte'
  import { tabs } from './lib/tabs.svelte'

  onMount(() => {
    void appState.load()

    const stopListening = listen<string>('directory-changed', (event) => {
      if (event.payload === tabs.active.directoryPath) tabs.active.reload()
    })

    return () => {
      void stopListening.then((stop) => stop())
      void invoke('unwatch_directory')
    }
  })

  // Only the visible folder is watched; switching tabs re-points the watcher.
  $effect(() => {
    const path = tabs.active.directoryPath

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
</div>
