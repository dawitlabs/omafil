<script lang="ts">
  import { onMount } from 'svelte'
  import Header from './lib/components/Header/Header.svelte'
  import Home from './lib/components/Home/Home.svelte'
  import Sidebar from './lib/components/Sidebar/Sidebar.svelte'
  import { appState } from './lib/appState.svelte'

  onMount(() => {
    void appState.load()
  })
</script>

<div class="app-shell">
  <Header />
  <Sidebar />

  <main class="app-main">
    <Home />
  </main>
</div>
