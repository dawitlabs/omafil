import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { tabs } from './tabs.svelte'
import { Navigation } from './navigation.svelte'
import { fileOperations } from './fileOperations.svelte'
import { readableError } from './errors'

type OpenRequest = {
  targets: { folder: string; selection: string[]; properties: string | null }[]
  error: string | null
}

function openRequest(request: OpenRequest) {
  if (request.error) fileOperations.error = request.error
  for (const target of request.targets) {
    let navigation = tabs.all.find((tab) => tab.isCurrentPath(target.folder))
    if (navigation) {
      tabs.select(tabs.all.indexOf(navigation))
      tabs.focus(navigation)
    } else if (tabs.all.length === 1 && tabs.tab.view.kind === 'home') {
      navigation = tabs.tab
    } else {
      navigation = new Navigation()
      if (!tabs.open(navigation)) return
    }
    navigation.reveal(target.folder, target.selection)
    if (target.properties) fileOperations.showProperties(target.properties)
  }
}

/** Register before draining: both early launch requests and later wake events
 * use the same backend queue, so neither a startup race nor duplicate wakes lose work. */
export async function listenForDesktopOpening(): Promise<() => void> {
  let stopped = false
  let draining = Promise.resolve()
  const drain = () => {
    draining = draining.then(async () => {
      if (stopped) return
      const requests = await invoke<OpenRequest[]>('take_open_requests')
      if (!stopped) for (const request of requests) openRequest(request)
    }).catch((error: unknown) => {
      fileOperations.error = readableError(error, 'Unable to open the requested location.')
    })
  }
  const stop = await listen('open-requested', drain)
  const stopErrors = await listen<string>('open-request-error', ({ payload }) => { fileOperations.error = payload })
  drain()
  return () => { stopped = true; stop(); stopErrors() }
}
