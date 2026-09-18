<script lang="ts">
  import DismissIcon from '@fluentui/svg-icons/icons/dismiss_20_regular.svg?no-inline'
  import { sharing } from '../../sharing.svelte'

  let dialog = $state<HTMLDialogElement | null>(null)

  function close() {
    dialog?.close()
    sharing.close()
  }

  const names = $derived((sharing.paths ?? []).map((path) => path.split('/').filter(Boolean).at(-1) ?? path))

  function modal(node: HTMLDialogElement) {
    node.showModal()
    return { destroy: () => node.close() }
  }
</script>

<dialog bind:this={dialog} use:modal class="dialog dialog--narrow share-dialog" aria-labelledby="share-title" oncancel={(event) => { event.preventDefault(); close() }} onkeydown={(event) => event.stopPropagation()}>
  <header class="dialog__header">
    <h2 id="share-title">Share</h2>
    <button type="button" aria-label="Close sharing" onclick={close}>
      <span class="masked-icon" style="--icon: url({DismissIcon})" aria-hidden="true"></span>
    </button>
  </header>

  <div class="share-dialog__body">
    <p class="share-dialog__selection">{names.length} {names.length === 1 ? 'item selected' : 'items selected'}</p>
    <ul class="share-dialog__files" aria-label="Items to share">
      {#each names.slice(0, 8) as name, index (index)}
        <li title={name}>{name}</li>
      {/each}
    </ul>
    {#if names.length > 8}<p class="share-dialog__more">And {names.length - 8} more</p>{/if}

    <section class="share-dialog__method" aria-labelledby="share-airdrop-title" aria-busy={sharing.loading}>
      <div class="share-dialog__method-heading">
        <h3 id="share-airdrop-title">AirDrop</h3>
        <span class="share-dialog__badge">{sharing.loading ? 'Checking' : 'Unavailable'}</span>
      </div>
      {#if sharing.loading}
        <p role="status">Checking availability…</p>
      {:else if sharing.error}
        <p role="alert">{sharing.error}</p>
        <button type="button" onclick={() => sharing.check()}>Try again</button>
      {:else}
        <p role="status">{sharing.availability?.reason.message ?? 'AirDrop is not available in this version of Omafil.'}</p>
        <p class="share-dialog__hint">Your selected files have not been sent.</p>
      {/if}
    </section>
  </div>

  <footer class="dialog__footer">
    <button type="button" onclick={close}>Close</button>
  </footer>
</dialog>
