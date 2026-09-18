<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { appState } from '../../appState.svelte'
  import { ServiceTask } from '../../linuxServices.svelte'
  import { tabs } from '../../tabs.svelte'
  import { readableError } from '../../errors'
  import type { DirectoryEntry } from '../../navigation.svelte'
  import { formatBytes, parentFolder } from '../../format'

  const task = new ServiceTask()
  let query = $state('')
  let scope = $state('')
  let entries = $state<DirectoryEntry[]>([])
  let searched = $state(false)
  let truncated = $state(false)
  let skipped = $state(0)
  let type = $state('all')
  let since = $state('')
  let resultHeading = $state<HTMLHeadingElement | null>(null)
  const extensions: Record<string, string[]> = {
    documents: ['pdf', 'doc', 'docx', 'odt', 'txt', 'md', 'rtf'],
    images: ['png', 'jpg', 'jpeg', 'webp', 'svg', 'gif', 'avif'],
    audio: ['mp3', 'wav', 'ogg', 'flac', 'm4a'],
  }
  const visible = $derived(entries.filter((entry) => {
    const extension = entry.name.split('.').at(-1)?.toLowerCase() ?? ''
    const cutoff = since ? new Date(`${since}T00:00:00`).getTime() / 1000 : null
    return (type === 'all' || extensions[type]?.includes(extension)) && (cutoff === null || (entry.modified !== null && entry.modified >= cutoff))
  }))

  async function search() {
    entries = []
    searched = false
    truncated = false
    skipped = 0
    const results = await task.run<{ entries: DirectoryEntry[]; truncated: boolean; skipped: number }>('indexed-search', {
      path: scope, query, showHidden: appState.settings.showHidden,
    })
    if (!results) return
    entries = results.entries
    truncated = results.truncated
    skipped = results.skipped
    searched = true
    resultHeading?.focus()
  }

  async function open(entry: DirectoryEntry) {
    try { await invoke('open_path', { path: entry.path }) }
    catch (error) { task.error = readableError(error, 'This item is no longer available.') }
  }

  onMount(() => {
    let disposed = false
    void invoke<string>('resolve_location', { location: 'home' }).then((home) => { if (!disposed) scope = home }).catch(() => {})
    return () => { disposed = true; task.dispose() }
  })
</script>

<section class="service-view" aria-labelledby="content-search-heading">
  <h1 id="content-search-heading">Content search</h1>
  <p class="service-view__hint" id="index-coverage">Search indexed document text and metadata with LocalSearch. Only indexed locations and supported file formats are included. Changes can take time to appear. This does not scan every file.</p>
  <form class="service-view__connect" onsubmit={(event) => { event.preventDefault(); if (!task.busy) void search() }}>
    <label for="content-query">Words in documents</label>
    <input id="content-query" bind:value={query} required maxlength="500" disabled={task.busy} aria-describedby="index-coverage" />
    <label for="content-scope">Folder</label>
    <input id="content-scope" bind:value={scope} required disabled={task.busy} />
    <button type="submit" disabled={task.busy || !query.trim() || !scope}>Search contents</button>
    {#if task.busy}<button type="button" onclick={() => task.cancel()}>Cancel search</button>{/if}
  </form>
  <div class="service-view__toolbar">
    <label for="content-type">Type</label>
    <select id="content-type" bind:value={type}><option value="all">All types</option><option value="documents">Documents</option><option value="images">Images</option><option value="audio">Audio</option></select>
    <label for="content-since">Modified since</label><input id="content-since" type="date" bind:value={since} />
    <button type="button" disabled={!query.trim() || !scope || task.busy} onclick={() => { void tabs.active.search(query, scope) }}>Search filenames instead</button>
  </div>
  {#if task.error}<p class="service-view__error" role="alert">{task.error}</p>{/if}
  <h2 tabindex="-1" bind:this={resultHeading}>Results</h2>
  <p role="status" aria-live="polite">{task.busy ? 'Searching the index…' : searched ? `${visible.length} results${skipped ? `; ${skipped} unavailable entries skipped` : ''}.` : 'Enter words and choose a folder.'}</p>
  {#if truncated}<p role="status">The index returned its first 500 matches. Filters apply to these matches. Narrow the folder or query for a complete result set.</p>{/if}
  {#if searched}
    <table class="service-view__table" aria-label="Indexed search results">
      <thead><tr><th scope="col">Name</th><th scope="col">Folder</th><th scope="col">Size</th><th scope="col">Actions</th></tr></thead>
      <tbody>
        {#each visible as entry (entry.path)}
          <tr><td><button type="button" onclick={() => open(entry)}>{entry.name}</button></td><td>{parentFolder(entry.path)}</td><td>{formatBytes(entry.size)}</td><td><button type="button" aria-label={`Show ${entry.name} in folder`} onclick={() => tabs.active.reveal(entry.path.slice(0, entry.path.lastIndexOf('/')) || '/', [entry.path])}>Show in folder</button></td></tr>
        {:else}<tr><td colspan="4">No indexed matches. Try different words, a wider folder, or filename search.</td></tr>{/each}
      </tbody>
    </table>
  {/if}
</section>
