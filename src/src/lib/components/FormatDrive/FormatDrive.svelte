<script lang="ts">
  import DismissIcon from '@fluentui/svg-icons/icons/dismiss_20_regular.svg?no-inline'
  import { driveStore } from '../../drives.svelte'
  import { formatBytes } from '../../format'
  import type { DriveInfo } from '../../navigation.svelte'

  let { drive, onclose }: { drive: DriveInfo; onclose: () => void } = $props()

  const filesystems = [
    { id: 'exfat', label: 'exFAT — every operating system, large files' },
    { id: 'vfat', label: 'FAT32 — every operating system, files under 4 GB' },
    { id: 'ntfs', label: 'NTFS — Windows' },
    { id: 'ext4', label: 'ext4 — Linux' },
    { id: 'btrfs', label: 'btrfs — Linux, with snapshots' },
  ]

  let filesystem = $state('exfat')
  let label = $state('')
  let isFormatting = $state(false)
  let error = $state<string | null>(null)

  $effect(() => {
    label = drive.name
  })

  async function startFormat() {
    isFormatting = true
    error = null

    const failure = await driveStore.format(drive, filesystem, label.trim())
    isFormatting = false

    if (failure) error = failure
    else onclose()
  }
</script>

<div class="dialog-backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && !isFormatting && onclose()} onkeydown={(event) => event.key === 'Escape' && !isFormatting && onclose()}>
  <dialog open class="dialog dialog--narrow" aria-labelledby="format-drive-title">
    <header class="dialog__header">
      <h2 id="format-drive-title">Format drive</h2>
      <button type="button" aria-label="Close" disabled={isFormatting} onclick={onclose}>
        <span class="masked-icon" style="--icon: url({DismissIcon})" aria-hidden="true"></span>
      </button>
    </header>

    <div class="format-drive__body">
      <p class="format-drive__warning" role="alert">
        Formatting erases everything on <strong>{drive.name || drive.device}</strong>
        ({drive.device}, {formatBytes(drive.totalBytes)}). This cannot be undone.
      </p>

      <label class="format-drive__field" for="format-filesystem">
        Filesystem
        <select id="format-filesystem" bind:value={filesystem} disabled={isFormatting}>
          {#each filesystems as option (option.id)}
            <option value={option.id}>{option.label}</option>
          {/each}
        </select>
      </label>

      <label class="format-drive__field" for="format-label">
        Drive name
        <input id="format-label" type="text" bind:value={label} disabled={isFormatting} placeholder="Optional" />
      </label>

      {#if error}
        <p class="format-drive__error" role="alert">{error}</p>
      {/if}
    </div>

    <footer class="dialog__footer">
      <button type="button" disabled={isFormatting} onclick={onclose}>Cancel</button>
      <button type="button" class="dialog__danger" disabled={isFormatting} onclick={startFormat}>
        {isFormatting ? 'Formatting…' : 'Erase and format'}
      </button>
    </footer>
  </dialog>
</div>
