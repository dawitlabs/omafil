<script lang="ts">
  import { onMount } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'

  type IntegrationStatus = {
    isDefault: boolean
    canEnable: boolean
    canRestore: boolean
    activationInstalled: boolean
    serviceActive: boolean
    reason: string | null
  }

  let status = $state<IntegrationStatus | null>(null)
  let busy = $state(false)
  let error = $state<string | null>(null)
  let message = $state<string | null>(null)

  async function refresh() {
    busy = true
    error = null
    try {
      status = await invoke<IntegrationStatus>('desktop_integration_status')
    } catch (caught) {
      error = typeof caught === 'string' ? caught : 'Unable to check desktop integration.'
    } finally {
      busy = false
    }
  }

  async function configure(enabled: boolean) {
    busy = true
    error = null
    message = null
    try {
      status = await invoke<IntegrationStatus>('configure_desktop_integration', { enabled })
      message = enabled
        ? 'Folder opening and Show in folder are configured for Omafil.'
        : 'Previous setup restored. Choices changed outside Omafil were kept.'
    } catch (caught) {
      error = typeof caught === 'string' ? caught : 'Unable to change desktop integration.'
    } finally {
      busy = false
    }
  }

  onMount(() => { void refresh() })
</script>

<section class="settings__group" aria-labelledby="settings-desktop-integration" aria-busy={busy}>
  <h2 id="settings-desktop-integration" class="settings__heading">Desktop integration</h2>
  <p class="settings__hint">Open folders and reveal downloaded files in Omafil, including when it is closed. Your previous setup is saved so you can restore it.</p>

  {#if status}
    <div class="settings__row">
      <div class="settings__label">
        <span>{status.isDefault ? 'Omafil opens your folders' : 'Another app opens your folders'}</span>
        <span class="settings__hint">{status.activationInstalled ? 'Show in folder can launch Omafil automatically.' : 'Automatic Show in folder launching is not configured.'}</span>
      </div>
      {#if !status.canRestore}
        <button class="settings__button" type="button" disabled={busy || !status.canEnable} onclick={() => configure(true)}>Make default</button>
      {:else}
        <button class="settings__button" type="button" disabled={busy} onclick={() => configure(false)}>Restore previous setup</button>
      {/if}
    </div>
    {#if status.reason && !status.canRestore}<p class="settings__hint">{status.reason}</p>{/if}
    {#if status.activationInstalled && !status.serviceActive}
      <p class="settings__hint">Close the other file manager and restart Omafil to activate Show in folder in this session.</p>
    {/if}
    {#if status.canRestore && !status.isDefault}
      <p class="settings__hint">Your folder handler changed outside Omafil. Restore previous setup to also remove Omafil’s saved Show in folder registration.</p>
    {/if}
  {:else if busy}
    <p class="settings__hint" role="status">Checking desktop integration…</p>
  {/if}

  {#if message}<p class="settings__hint" role="status">{message}</p>{/if}
  {#if error}
    <p class="settings__error" role="alert">{error}</p>
    <button class="settings__button" type="button" disabled={busy} onclick={refresh}>Check again</button>
  {/if}
</section>
