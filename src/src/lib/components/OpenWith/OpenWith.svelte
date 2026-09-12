<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import DismissIcon from '@fluentui/svg-icons/icons/dismiss_20_regular.svg?no-inline'
  import { readableError } from '../../errors'

  type Opener = { id: string; name: string; isDefault: boolean }

  let { path, onclose }: { path: string; onclose: () => void } = $props()

  let openers = $state<Opener[]>([])
  let isLoading = $state(true)
  let error = $state<string | null>(null)
  let list = $state<HTMLUListElement | null>(null)

  const fileName = $derived(path.split('/').filter(Boolean).at(-1) ?? path)

  $effect(() => {
    isLoading = true
    error = null

    invoke<Opener[]>('list_openers', { path })
      .then((found) => {
        openers = found
      })
      .catch((caught: unknown) => {
        error = readableError(caught, 'Unable to find apps for this file.')
      })
      .finally(() => {
        isLoading = false
      })
  })

  $effect(() => {
    if (!isLoading) list?.querySelector('button')?.focus()
  })

  async function launch(id: string) {
    error = null

    try {
      await invoke('open_with', { path, desktopId: id })
      onclose()
    } catch (caught) {
      error = readableError(caught, 'Unable to start that app.')
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') onclose()
  }
</script>

<div class="open-with__backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && onclose()} onkeydown={handleKeydown}>
  <dialog open class="open-with" aria-labelledby="open-with-title">
    <header class="open-with__header">
      <h2 id="open-with-title">Open with</h2>
      <button type="button" aria-label="Close" onclick={onclose}>
        <span class="masked-icon" style="--icon: url({DismissIcon})" aria-hidden="true"></span>
      </button>
    </header>
    <p class="open-with__file">{fileName}</p>

    {#if isLoading}
      <p class="open-with__state">Finding apps…</p>
    {:else if error}
      <p class="open-with__state open-with__state--error" role="alert">{error}</p>
    {:else if openers.length === 0}
      <p class="open-with__state">No installed app claims this file type. Use Edit to open it in your editor.</p>
    {:else}
      <ul class="open-with__list" bind:this={list}>
        {#each openers as opener (opener.id)}
          <li>
            <button type="button" class="open-with__app" onclick={() => launch(opener.id)}>
              <span>{opener.name}</span>
              {#if opener.isDefault}<span class="open-with__default">Default</span>{/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}

    <footer class="open-with__footer"><button type="button" onclick={onclose}>Cancel</button></footer>
  </dialog>
</div>
