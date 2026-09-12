<script lang="ts">
  import DismissIcon from '@fluentui/svg-icons/icons/dismiss_20_regular.svg?no-inline'
  import { isApplicable, planBulkRename } from '../../bulkRename'
  import { fileOperations } from '../../fileOperations.svelte'

  let { paths, onclose }: { paths: string[]; onclose: () => void } = $props()

  let find = $state('')
  let replace = $state('')
  let pattern = $state('{name}{ext}')
  let start = $state(1)
  let findInput = $state<HTMLInputElement | null>(null)

  const names = $derived(paths.map((path) => path.split('/').filter(Boolean).at(-1) ?? path))
  const taken = $derived(fileOperations.entries.map((entry) => entry.name))
  const plan = $derived(planBulkRename(names, { find, replace, pattern, start: Number.isFinite(start) ? start : 1 }, taken))
  const canApply = $derived(isApplicable(plan) && !fileOperations.isBusy)

  $effect(() => {
    findInput?.focus()
  })

  function handleSubmit(event: SubmitEvent) {
    event.preventDefault()
    if (!canApply) return

    const items = plan.flatMap((item, index) => (item.from === item.to ? [] : [{ path: paths[index], to: item.to }]))
    void fileOperations.applyBulkRename(items)
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') onclose()
  }
</script>

<div class="dialog-backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && onclose()} onkeydown={handleKeydown}>
  <dialog open class="dialog" aria-labelledby="bulk-rename-title">
    <form onsubmit={handleSubmit}>
      <header class="dialog__header">
        <h2 id="bulk-rename-title">Rename {paths.length} items</h2>
        <button type="button" aria-label="Close" onclick={onclose}>
          <span class="masked-icon" style="--icon: url({DismissIcon})" aria-hidden="true"></span>
        </button>
      </header>

      <div class="bulk-rename__fields">
        <label class="bulk-rename__field">
          <span>Find</span>
          <input type="text" bind:value={find} bind:this={findInput} placeholder="Text to replace" autocomplete="off" spellcheck="false" />
        </label>
        <label class="bulk-rename__field">
          <span>Replace with</span>
          <input type="text" bind:value={replace} autocomplete="off" spellcheck="false" />
        </label>
        <label class="bulk-rename__field bulk-rename__field--wide">
          <span>Pattern</span>
          <input type="text" bind:value={pattern} autocomplete="off" spellcheck="false" />
          <span class="bulk-rename__hint">{'{name}'} keeps the name, {'{ext}'} the extension, {'{n}'} counts up.</span>
        </label>
        <label class="bulk-rename__field bulk-rename__field--small">
          <span>Start at</span>
          <input type="number" bind:value={start} min="0" step="1" />
        </label>
      </div>

      <table class="bulk-rename__preview">
        <thead><tr><th scope="col">Current</th><th scope="col">New</th></tr></thead>
        <tbody>
          {#each plan as item (item.from)}
            <tr class:bulk-rename__row--issue={item.issue !== null} class:bulk-rename__row--same={item.from === item.to}>
              <td>{item.from}</td>
              <td>{item.to}{#if item.issue}<span class="bulk-rename__issue">{item.issue}</span>{/if}</td>
            </tr>
          {/each}
        </tbody>
      </table>

      <footer class="dialog__footer">
        <button type="button" onclick={onclose}>Cancel</button>
        <button type="submit" class="dialog__primary" disabled={!canApply}>Rename</button>
      </footer>
    </form>
  </dialog>
</div>
