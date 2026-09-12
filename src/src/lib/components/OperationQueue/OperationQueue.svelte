<script lang="ts">
  import { fileOperations } from '../../fileOperations.svelte'
  import { formatBytes } from '../../format'
  import { isTerminal, operationPercent, type FileOperation } from '../../operationState'
  import './operation-queue.css'

  const titles: Record<FileOperation['kind'], string> = {
    copy: 'Copy files', move: 'Move files', compress: 'Create ZIP archive', extract: 'Extract ZIP archive',
  }

  function status(operation: FileOperation): string {
    if (operation.state === 'completed') return 'Completed'
    if (operation.state === 'cancelled') return 'Cancelled · completed items were kept'
    if (operation.state === 'failed') return 'Could not finish · completed items were kept'
    if (operation.cancellationRequested) return 'Cancelling…'
    if (operation.state === 'queued') return 'Waiting to start…'
    return operation.totalBytes === null ? 'Preparing…' : 'In progress'
  }
</script>

{#if fileOperations.operations.length > 0}
  <section class="operation-queue" aria-label="File operations">
    {#each fileOperations.operations as operation (operation.id)}
      <article class="operation-queue__item" aria-label={titles[operation.kind]}>
        <div class="operation-queue__heading">
          <strong>{titles[operation.kind]}</strong>
          {#if isTerminal(operation)}
            <button type="button" onclick={() => fileOperations.dismissOperation(operation.id)}>Dismiss</button>
          {:else}
            <button type="button" disabled={operation.cancellationRequested} onclick={() => fileOperations.cancelOperation(operation.id)}>
              {operation.cancellationRequested ? 'Cancelling…' : 'Cancel'}
            </button>
          {/if}
        </div>
        <p class="operation-queue__status" aria-live="polite">{status(operation)}</p>
        {#if operation.currentName}
          <p class="operation-queue__name" title={operation.currentName}>{operation.currentName}</p>
        {/if}
        {#if operation.totalBytes !== null}
          <p class="operation-queue__detail">{formatBytes(operation.completedBytes)} of {formatBytes(operation.totalBytes)} processed</p>
        {/if}
        {#if operation.kind === 'copy' || operation.kind === 'move'}
          <p class="operation-queue__detail">{operation.completedItems} of {operation.totalItems} items handled{operation.results?.some((result) => result.skipped) ? ` · ${operation.results.filter((result) => result.skipped).length} skipped` : ''}</p>
        {/if}
        {#if operation.error || operation.cancellationError}
          <p class="operation-queue__message" role="alert">{operation.error ?? operation.cancellationError}</p>
        {/if}
        {#if operation.state !== 'cancelled' && operation.state !== 'failed'}
          <progress class="operation-queue__progress" aria-label={`${titles[operation.kind]} progress`} max="100" value={operationPercent(operation)}></progress>
        {/if}
      </article>
    {/each}
  </section>
{/if}
