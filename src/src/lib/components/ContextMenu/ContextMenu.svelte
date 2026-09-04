<script lang="ts">
  export type ContextMenuItem =
    | { kind: 'separator' }
    | { kind: 'heading'; label: string }
    | { kind: 'action'; label: string; shortcut?: string; disabled?: boolean; onSelect: () => void }
    | { kind: 'toggle'; label: string; checked: boolean; onSelect: () => void }

  let {
    x,
    y,
    items,
    onclose,
  }: {
    x: number
    y: number
    items: ContextMenuItem[]
    onclose: () => void
  } = $props()

  let menu = $state<HTMLElement | null>(null)

  const position = $derived.by(() => {
    const width = menu?.offsetWidth ?? 220
    const height = menu?.offsetHeight ?? 0

    return {
      left: Math.min(x, window.innerWidth - width - 8),
      top: Math.min(y, Math.max(8, window.innerHeight - height - 8)),
    }
  })

  function choose(item: Extract<ContextMenuItem, { kind: 'action' | 'toggle' }>) {
    if (item.kind === 'action' && item.disabled) return

    onclose()
    item.onSelect()
  }
</script>

<svelte:window
  onkeydown={(event) => event.key === 'Escape' && onclose()}
  onresize={onclose}
/>

<div class="context-menu__backdrop" onpointerdown={onclose} oncontextmenu={(event) => event.preventDefault()} role="presentation"></div>

<div bind:this={menu} class="context-menu" style="left: {position.left}px; top: {position.top}px" role="menu" tabindex="-1">
  {#each items as item, index (index)}
    {#if item.kind === 'separator'}
      <hr class="context-menu__separator" />
    {:else if item.kind === 'heading'}
      <p class="context-menu__heading">{item.label}</p>
    {:else if item.kind === 'toggle'}
      <button class="context-menu__item" type="button" role="menuitemcheckbox" aria-checked={item.checked} onclick={() => choose(item)}>
        <span>{item.label}</span>
        <span class="context-menu__check" class:context-menu__check--on={item.checked} aria-hidden="true"></span>
      </button>
    {:else}
      <button class="context-menu__item" type="button" role="menuitem" disabled={item.disabled} onclick={() => choose(item)}>
        <span>{item.label}</span>
        {#if item.shortcut}<span class="context-menu__shortcut">{item.shortcut}</span>{/if}
      </button>
    {/if}
  {/each}
</div>
