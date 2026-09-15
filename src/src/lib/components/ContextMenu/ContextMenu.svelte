<script lang="ts">
  import { viewport } from '../../displayScale'
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

  const MARGIN = 8

  const placement = $derived.by(() => {
    const { width: available, height: room } = viewport()
    const width = menu?.offsetWidth ?? 220
    const height = menu?.offsetHeight ?? 0

    return {
      left: Math.max(MARGIN, Math.min(x, available - width - MARGIN)),
      top: Math.max(MARGIN, Math.min(y, room - height - MARGIN)),
      // A menu with more entries than the window is tall scrolls rather than
      // running off the bottom edge.
      maxHeight: room - MARGIN * 2,
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

<div bind:this={menu} class="context-menu" style="left: {placement.left}px; top: {placement.top}px; max-height: {placement.maxHeight}px" role="menu" tabindex="-1">
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
