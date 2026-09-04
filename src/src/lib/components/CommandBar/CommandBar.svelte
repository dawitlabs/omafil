<script lang="ts">
  import { fileOperations } from '../../fileOperations.svelte'

  const hasSelection = $derived(fileOperations.selectedPaths.length > 0)
  const hasSingleSelection = $derived(fileOperations.selectedPaths.length === 1)
</script>

<div class="file-commands" role="toolbar" aria-label="File commands">
  <button class="file-commands__button" type="button" onclick={() => fileOperations.createFolder()}>New folder</button>
  <span class="file-commands__divider" aria-hidden="true"></span>
  <button class="file-commands__button" type="button" disabled={!hasSelection} onclick={() => fileOperations.cutSelection()}>Cut</button>
  <button class="file-commands__button" type="button" disabled={!hasSelection} onclick={() => fileOperations.copySelection()}>Copy</button>
  <button class="file-commands__button" type="button" disabled={!fileOperations.canPaste} onclick={() => fileOperations.paste()}>Paste</button>
  <span class="file-commands__divider" aria-hidden="true"></span>
  <button class="file-commands__button" type="button" disabled={!hasSingleSelection} onclick={() => fileOperations.startRenaming()}>Rename</button>
  <button class="file-commands__button" type="button" disabled={!hasSelection} onclick={() => fileOperations.deleteSelection()}>Delete</button>
</div>
