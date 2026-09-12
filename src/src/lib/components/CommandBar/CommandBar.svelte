<script lang="ts">
  import { fileOperations } from '../../fileOperations.svelte'
  import { tabs } from '../../tabs.svelte'

  const inFolder = $derived(fileOperations.directoryPath !== '')
  const hasSelection = $derived(fileOperations.selectedPaths.length > 0)
  const hasSingleSelection = $derived(fileOperations.selectedPaths.length === 1)
</script>

<div class="file-commands" role="toolbar" aria-label="File commands">
  <button class="file-commands__button" type="button" disabled={!inFolder} onclick={() => fileOperations.createFolder()}>New folder</button>
  <span class="file-commands__divider" aria-hidden="true"></span>
  <button class="file-commands__button" type="button" disabled={!hasSelection} onclick={() => fileOperations.cutSelection()}>Cut</button>
  <button class="file-commands__button" type="button" disabled={!hasSelection} onclick={() => fileOperations.copySelection()}>Copy</button>
  <button class="file-commands__button" type="button" disabled={!fileOperations.canPaste} onclick={() => fileOperations.paste()}>Paste</button>
  <span class="file-commands__divider" aria-hidden="true"></span>
  <button class="file-commands__button" type="button" disabled={!hasSingleSelection} onclick={() => fileOperations.startRenaming()}>Rename</button>
  <button class="file-commands__button" type="button" disabled={!hasSelection} onclick={() => fileOperations.deleteSelection()}>Delete</button>
  <button class="file-commands__button" type="button" disabled={fileOperations.selectedPaths.length < 2} onclick={() => fileOperations.compressSelection()}>Compress</button>
  <button class="file-commands__button" type="button" disabled={!hasSingleSelection} onclick={() => fileOperations.extractSelection()}>Extract</button>
  <span class="file-commands__divider" aria-hidden="true"></span>
  <button class="file-commands__button" type="button" disabled={!hasSingleSelection} onclick={() => fileOperations.showProperties()}>Properties</button>
  <button class="file-commands__button" type="button" disabled={!inFolder} title="Open a terminal in this folder (F4)" onclick={() => tabs.active.openTerminal()}>Terminal</button>
  <span class="file-commands__divider" aria-hidden="true"></span>
  <button class="file-commands__button" type="button" aria-pressed={fileOperations.viewMode === 'details'} onclick={() => (fileOperations.viewMode = 'details')}>Details</button>
  <button class="file-commands__button" type="button" aria-pressed={fileOperations.viewMode === 'icons'} onclick={() => (fileOperations.viewMode = 'icons')}>Icons</button>
  <button class="file-commands__button" type="button" aria-pressed={fileOperations.viewMode === 'preview'} onclick={() => (fileOperations.viewMode = 'preview')}>Preview</button>
</div>
