<script lang="ts">
  import { onMount, tick } from 'svelte'
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'
  import { appState } from '../../appState.svelte'
  import { readableError } from '../../errors'
  import { formatBytes } from '../../format'
  import { ServiceTask, displayAddress, type RemoteListing, type ServiceMount, type CopyReport, type ServiceLocation } from '../../linuxServices.svelte'

  const task = new ServiceTask()
  const discovery = new ServiceTask()
  const probe = new ServiceTask()
  let address = $state('')
  let remember = $state(false)
  let mounted = $state<ServiceMount[]>([])
  let listing = $state<RemoteListing | null>(null)
  let history = $state<string[]>([])
  let selected = $state<string[]>([])
  let report = $state<CopyReport | null>(null)
  let status = $state('')
  let heading = $state<HTMLHeadingElement | null>(null)
  let installed = $state<string[] | null>(null)

  // Addresses the connect dialog accepts, and the phone backends that put a
  // device in the list. Until `capabilities` answers, assume everything works
  // rather than hiding a protocol this system may well support.
  const CONNECT_SCHEMES: string[] = ['sftp', 'smb', 'dav', 'davs', 'ftp', 'ftps']
  const PHONE_SCHEMES = ['afc', 'mtp', 'gphoto2']
  const SCHEME_LABELS: Record<string, string> = {
    sftp: 'SFTP', smb: 'SMB', dav: 'WebDAV', davs: 'WebDAV', ftp: 'FTP', ftps: 'FTPS',
  }

  const available = $derived.by(() => {
    const schemes = installed
    return schemes ? CONNECT_SCHEMES.filter((scheme) => schemes.includes(scheme)) : CONNECT_SCHEMES
  })
  const hasPhoneBackend = $derived.by(() => {
    const schemes = installed
    return !schemes || PHONE_SCHEMES.some((scheme) => schemes.includes(scheme))
  })

  function sentenceList(items: string[]): string {
    return items.length < 2 ? items.join('') : `${items.slice(0, -1).join(', ')} and ${items.at(-1)}`
  }

  const protocolNames = $derived(sentenceList([...new Set(available.map((scheme) => SCHEME_LABELS[scheme]))]))
  const protocolHelp = $derived(
    available.length === 0
      ? 'No network protocols are available. Install a GVfs backend such as gvfs or gvfs-smb to connect to servers.'
      : `${protocolNames} ${available.length === 1 ? 'is' : 'are'} available from the installed GVfs backends. Enter passwords only in the desktop sign-in dialog.${available.includes('ftp') ? ' FTP sends data without encryption.' : ''}`,
  )
  const addressPlaceholder = $derived(
    available.length === 0 ? 'No network backend installed' : available.slice(0, 2).map((scheme) => `${scheme}://server/path`).join(' or '),
  )

  function submit() {
    const uri = address.trim()
    const scheme = uri.split(':', 1)[0].toLowerCase()
    if (available.includes(scheme)) return void connect(uri)
    task.error = available.length === 0
      ? 'No network protocols are available. Install a GVfs backend such as gvfs or gvfs-smb to connect to servers.'
      : `This system cannot connect with ${scheme ? `${scheme}://` : 'that address'}. Use ${sentenceList(available.map((entry) => `${entry}://`))}.`
  }

  async function refreshDevices() {
    const devices = await discovery.run<ServiceMount[]>('mounts')
    if (devices) mounted = devices
  }

  async function browse(uri: string, back = false) {
    const next = await task.run<RemoteListing>('list', { location: { kind: 'remote', uri }, showHidden: appState.settings.showHidden })
    if (!next) return
    if (listing && !back && listing.uri !== next.uri) history = [...history, listing.uri]
    listing = next
    selected = []
    status = `${next.entries.length} items loaded${next.truncated ? '; more items exist. Narrow the location to see them.' : '.'}`
    await tick()
    heading?.focus()
  }

  async function connect(uri: string, device = false) {
    report = null
    const result = await task.run<{ uri: string }>('connect', { uri, device })
    if (!result) return
    if (remember && !device) appState.update({ serverUris: [...new Set([...appState.settings.serverUris, uri])].slice(-16) })
    await browse(result.uri)
    void refreshDevices()
  }

  async function goBack() {
    const uri = history.at(-1)
    if (!uri) return
    await browse(uri, true)
    if (!task.error) history = history.slice(0, -1)
  }

  async function copy(sources: ServiceLocation[], destination: ServiceLocation) {
    report = null
    status = 'Copying. Keep the device connected until the transfer finishes.'
    const result = await task.run<CopyReport>('copy', { sources, destination })
    if (!result) { status = ''; return }
    report = result
    status = `${result.copied} selected items copied. ${result.failures.length} failed.`
    if (destination.kind === 'remote' && listing) await browse(listing.uri, true)
  }

  async function download() {
    try {
      const path = await invoke<string>('resolve_location', { location: 'downloads' })
      await copy(selected.map((uri) => ({ kind: 'remote', uri })), { kind: 'local', path })
    } catch (error) { task.error = readableError(error, 'The Downloads folder is unavailable.') }
  }

  async function paste() {
    if (!listing) return
    try {
      const clipboard = await invoke<[string[], boolean] | null>('read_file_clipboard')
      if (!clipboard?.[0].length) { task.error = 'Copy files or folders in Omafil or another file manager first.'; return }
      if (clipboard[1]) { task.error = 'Moving to a remote location is not supported yet. Copy the items instead of cutting them.'; return }
      await copy(clipboard[0].map((path) => ({ kind: 'local', path })), { kind: 'remote', uri: listing.uri })
    } catch (error) { task.error = readableError(error, 'The clipboard could not be read.') }
  }

  function toggle(uri: string) {
    selected = selected.includes(uri) ? selected.filter((item) => item !== uri) : [...selected, uri]
  }

  onMount(() => {
    // An unanswered probe leaves every protocol on offer, which is how this view
    // behaved before it asked.
    void probe.run<{ schemes: string[] }>('capabilities').then((found) => { if (found) installed = found.schemes })
    void refreshDevices()
    const stop = listen<{ id: string; update: { completed: number; total: number } }>('linux-service-progress', ({ payload }) => {
      if (payload.id === task.id) task.progress = payload.update
    })
    return () => { task.dispose(); discovery.dispose(); probe.dispose(); void stop.then((unlisten) => unlisten()) }
  })
</script>

<section class="service-view" aria-labelledby="network-heading">
  <h1 id="network-heading" tabindex="-1" bind:this={heading}>Network &amp; Devices</h1>
  <form class="service-view__connect" onsubmit={(event) => { event.preventDefault(); if (!task.busy) submit() }}>
    <label for="server-address">Server address</label>
    <input id="server-address" type="text" bind:value={address} placeholder={addressPlaceholder} autocomplete="off" spellcheck="false" required disabled={task.busy || available.length === 0} aria-describedby="server-help" />
    <button type="submit" disabled={task.busy || !address.trim() || available.length === 0}>Connect</button>
  </form>
  <label class="service-view__hint"><input type="checkbox" bind:checked={remember} /> Remember server address (no credentials)</label>
  <p id="server-help" class="service-view__hint">{protocolHelp}</p>

  {#if appState.settings.serverUris.length}
    <h2>Saved servers</h2>
    <ul class="service-view__mounts">{#each appState.settings.serverUris as uri (uri)}<li>
      <button type="button" disabled={task.busy} onclick={() => connect(uri)}>{displayAddress(uri)}</button>
      <button type="button" aria-label={`Forget ${displayAddress(uri)}`} onclick={() => appState.update({ serverUris: appState.settings.serverUris.filter((saved) => saved !== uri) })}>Forget</button>
    </li>{/each}</ul>
  {/if}
  <div class="service-view__toolbar" role="group" aria-label="Connected devices">
    <h2>Servers and phones</h2>
    <button type="button" disabled={discovery.busy} onclick={refreshDevices}>Refresh devices</button>
  </div>
  {#if discovery.error}<p role="alert">{discovery.error}</p>{/if}
  {#if discovery.busy}<p role="status">Looking for connected devices…</p>{/if}
  <ul class="service-view__mounts">
    {#each mounted as device (device.uri)}
      <li><button type="button" disabled={task.busy} onclick={() => device.mounted ? browse(device.uri) : connect(device.uri, true)}>{device.name} · {device.mounted ? 'Open' : 'Connect'}</button></li>
    {:else}
      {#if !discovery.busy}<li>{hasPhoneBackend ? 'Connect a server, or unlock your phone and enable USB file transfer.' : 'Connect a server. No phone backend is installed \u2014 add gvfs-mtp for Android or gvfs-afc for iPhone.'}</li>{/if}
    {/each}
  </ul>

  {#if task.error}<p class="service-view__error" role="alert">{task.error}</p>{/if}
  <p class="service-view__status" role="status" aria-live="polite">{task.busy ? task.progress ? `Copied ${task.progress.completed} of ${task.progress.total} selected items…` : 'Working…' : status}</p>
  {#if task.busy}<button type="button" onclick={() => task.cancel()}>Cancel operation</button>{/if}
  {#if report}
    <p>{report.copied} selected items copied.</p>
    {#if report.failures.length}<ul role="alert">{#each report.failures as failure, index (index)}<li>{failure.name}: {failure.message}</li>{/each}</ul>{/if}
  {/if}

  {#if listing}
    <div class="service-view__toolbar" role="group" aria-label="Remote folder actions">
      <button type="button" disabled={task.busy || !history.length} onclick={goBack}>Back</button>
      <button type="button" disabled={task.busy || !listing.parent} onclick={() => listing?.parent && browse(listing.parent)}>Up</button>
      <button type="button" disabled={task.busy} onclick={() => listing && browse(listing.uri, true)}>Refresh folder</button>
      <button type="button" disabled={task.busy || !selected.length} onclick={download}>Copy to Downloads</button>
      <button type="button" disabled={task.busy} onclick={paste}>Paste copied files here</button>
    </div>
    <p class="service-view__address">{displayAddress(listing.uri)}</p>
    <p class="service-view__hint">Copies keep existing destination names intact. Failed or cancelled transfers may leave incomplete files. Remote move, trash and Undo are not available yet.</p>
    {#if listing.truncated}<p role="status">Showing up to 1,000 items. This folder has more items than this view can show.</p>{/if}
    <table class="service-view__table" aria-label="Remote folder contents" aria-busy={task.busy}>
      <thead><tr><th scope="col">Select</th><th scope="col">Name</th><th scope="col">Size</th></tr></thead>
      <tbody>
        {#each listing.entries as entry (entry.uri)}
          <tr>
            <td><input type="checkbox" aria-label={`Select ${entry.name}`} checked={selected.includes(entry.uri)} disabled={task.busy || (!entry.isDirectory && !entry.isRegular)} onchange={() => toggle(entry.uri)} /></td>
            <td>{#if entry.isDirectory}<button type="button" disabled={task.busy} onclick={() => browse(entry.uri)} aria-label={`Open folder ${entry.name}`}>{entry.name}</button>{:else}{entry.name}{/if}</td>
            <td>{entry.isDirectory ? 'Folder' : formatBytes(entry.size)}</td>
          </tr>
        {:else}<tr><td colspan="3">This folder is empty.</td></tr>{/each}
      </tbody>
    </table>
  {/if}
</section>
