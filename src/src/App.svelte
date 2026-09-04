<script lang="ts">
  import Header from './lib/components/Header/Header.svelte'
  import Home from './lib/components/Home/Home.svelte'
  import Sidebar from './lib/components/Sidebar/Sidebar.svelte'
</script>

<div class="app-shell">
  <Header />
  <Sidebar />

  <main class="app-main">
    <Home />
  </main>
</div>
