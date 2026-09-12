<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { convertFileSrc } from '@tauri-apps/api/core'
  import DismissIcon from '@fluentui/svg-icons/icons/dismiss_20_regular.svg?no-inline'
  import FolderIcon from '@fluentui/svg-icons/icons/folder_48_regular.svg?no-inline'
  import { fileIcon } from '../../fileIcons'
  import { formatBytes, formatModified, typeLabel } from '../../format'

  type PathInspection = {
    name: string
    path: string
    entryType: 'file' | 'folder'
    size: number
    itemCount: number
    modified: number | null
    created: number | null
    preview: string | null
    previewTruncated: boolean
    mediaPreview: string | null
    mediaType: string | null
  }

  let { path, mode, onclose }: { path: string; mode: 'preview' | 'properties'; onclose?: () => void } = $props()

  let inspection = $state<PathInspection | null>(null)
  let error = $state<string | null>(null)
  let loading = $state(false)
  let activeTab = $state('General')

  async function load(target: string) {
    loading = true
    error = null
    inspection = null

    try {
      inspection = await invoke<PathInspection>('inspect_path', { path: target })
    } catch {
      error = 'Unable to read details for this item.'
    } finally {
      loading = false
    }
  }

  $effect(() => {
    void load(path)
  })

  const isFolder = $derived(inspection?.entryType === 'folder')
  const icon = $derived(inspection ? (isFolder ? { icon: FolderIcon, tone: 'var(--folder-body-bottom)' } : fileIcon(inspection.name)) : null)
  const mediaSource = $derived(inspection?.mediaType ? convertFileSrc(inspection.path) : null)
</script>

{#if mode === 'properties'}
  <div class="file-inspector__backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && onclose?.()}>
    <dialog open class="file-inspector file-inspector--dialog" aria-labelledby="properties-title">
      <header class="file-inspector__header">
        <h2 id="properties-title">Properties</h2>
        <button type="button" aria-label="Close properties" onclick={onclose}>
          <span class="masked-icon" style="--icon: url({DismissIcon})" aria-hidden="true"></span>
        </button>
      </header>
      <div class="file-inspector__tabs" role="tablist" aria-label="Properties sections">{#each ['General', 'Details', 'Permissions'] as tab}<button type="button" role="tab" aria-selected={activeTab === tab} onclick={() => activeTab = tab}>{tab}</button>{/each}</div>
      <div class="file-inspector__dialog-content">{@render content()}</div>
      <footer class="file-inspector__footer"><button type="button" onclick={onclose}>Close</button></footer>
    </dialog>
  </div>
{:else}
  <aside class="file-inspector file-inspector--preview" aria-label="Preview pane">
    <header class="file-inspector__header">
      <h2>Preview</h2>
      <button type="button" aria-label="Close preview pane" onclick={onclose}>
        <span class="masked-icon" style="--icon: url({DismissIcon})" aria-hidden="true"></span>
      </button>
    </header>
    {@render content()}
  </aside>
{/if}

{#snippet content()}
  {#if loading}
    <p class="file-inspector__state">Loading details…</p>
  {:else if error}
    <p class="file-inspector__state">{error}</p>
  {:else if inspection && icon}
    <div class="file-inspector__identity" class:file-inspector__identity--dialog={mode === 'properties'}>
      <span class="masked-icon file-inspector__icon" style="--icon: url({icon.icon}); color: {icon.tone}" aria-hidden="true"></span>
      <strong title={inspection.name}>{inspection.name}</strong>
      <span>{typeLabel(inspection.name, isFolder)}</span>
    </div>

    {#if mode !== 'properties' || activeTab === 'General'}<section class="file-inspector__section"><h3>General</h3><dl class="file-inspector__details">
      <div><dt>Type</dt><dd>{typeLabel(inspection.name, isFolder)}</dd></div>
      <div><dt>Location</dt><dd class="file-inspector__location" title={inspection.path}>{inspection.path}</dd></div>
      <div><dt>Size</dt><dd>{formatBytes(inspection.size)}</dd></div>
      {#if isFolder}<div><dt>Contains</dt><dd>{inspection.itemCount.toLocaleString()} items</dd></div>{/if}
    </dl></section>{/if}
    {#if mode === 'properties' && activeTab === 'Details'}<section class="file-inspector__section"><h3>Details</h3><dl class="file-inspector__details"><div><dt>File name</dt><dd>{inspection.name}</dd></div><div><dt>Path</dt><dd class="file-inspector__location">{inspection.path}</dd></div></dl></section>{/if}
    {#if mode === 'properties' && activeTab === 'Permissions'}<section class="file-inspector__section"><h3>Permissions</h3><p class="file-inspector__state">Permissions are managed by your Linux desktop and file system.</p></section>{/if}
    <section class="file-inspector__section"><h3>Dates</h3><dl class="file-inspector__details">
      <div><dt>Modified</dt><dd>{formatModified(inspection.modified)}</dd></div>
      <div><dt>Created</dt><dd>{formatModified(inspection.created)}</dd></div>
    </dl></section>

    {#if mode === 'preview'}
      {#if inspection.preview}
        <pre class="file-inspector__text-preview">{inspection.preview}</pre>
        {#if inspection.previewTruncated}<p class="file-inspector__state">Showing the first 48 KB.</p>{/if}
      {:else if mediaSource && inspection.mediaType?.startsWith('image/')}
        <img class="file-inspector__media-preview" src={mediaSource} alt={`Preview of ${inspection.name}`} />
      {:else if mediaSource && inspection.mediaType === 'application/pdf'}
        <iframe class="file-inspector__media-preview" title={`Preview of ${inspection.name}`} src={mediaSource}></iframe>
      {:else if mediaSource && inspection.mediaType?.startsWith('audio/')}
        <audio class="file-inspector__media-preview" controls src={mediaSource}></audio>
      {:else if mediaSource && inspection.mediaType?.startsWith('video/')}
        <!-- svelte-ignore a11y_media_has_caption -->
        <video class="file-inspector__media-preview" controls src={mediaSource}></video>
      {:else}
        <p class="file-inspector__state">A preview is not available for this {isFolder ? 'folder' : 'file type'}.</p>
      {/if}
    {/if}
  {/if}
{/snippet}
