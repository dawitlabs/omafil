// Golden-path checks against the preview harness, which mocks the Rust side.
// usage: bunx vite --port 1420   then   PLAYWRIGHT_DIR=<node_modules> node src/tests/e2e.mjs
import { createRequire } from 'node:module'
const require = createRequire(import.meta.url)
const { chromium } = require(require.resolve('playwright', { paths: [process.env.PLAYWRIGHT_DIR ?? process.cwd()] }))

const URL = process.env.PREVIEW_URL ?? 'http://localhost:1420/preview.html'
const failures = []
let checks = 0

function check(name, actual, expected) {
  checks += 1
  const got = JSON.stringify(actual)
  const want = JSON.stringify(expected)
  if (got === want) return console.log(`  ok  ${name}`)
  failures.push(name)
  console.log(`FAIL  ${name}\n        expected ${want}\n        actual   ${got}`)
}

const browser = await chromium.launch({ executablePath: process.env.CHROMIUM ?? '/usr/bin/chromium' })

async function scenario(name, body) {
  if (process.env.E2E_SCENARIO && !new RegExp(process.env.E2E_SCENARIO).test(name)) return
  const page = await browser.newPage({ viewport: { width: 1300, height: 800 } })
  const errors = []
  page.on('pageerror', (error) => errors.push(error.message))
  page.on('console', (message) => message.type() === 'error' && errors.push(message.text()))
  console.log(`\n${name}`)
  await page.goto(URL)
  await page.waitForTimeout(700)
  const rows = async () => (await page.locator('[data-path]').allInnerTexts()).map((text) => text.split('\n')[0])
  const openDocuments = async () => {
    await page.locator('.file-sidebar__item', { hasText: 'Documents' }).first().click()
    await page.waitForTimeout(700)
  }
  await body({ page, rows, openDocuments })
  check(`${name} — no console or page errors`, errors.filter((e) => !e.includes('404')), [])
  await page.close()
}

await scenario('navigation', async ({ rows, openDocuments }) => {
  await openDocuments()
  check('lists the folder, hidden files excluded', await rows(), ['budget.xlsx', 'notes.txt', 'read.pdf'])
})

await scenario('undo a delete', async ({ page, rows, openDocuments }) => {
  await openDocuments()
  await page.locator('[data-path]', { hasText: 'notes.txt' }).first().click()
  await page.getByRole('button', { name: 'Delete', exact: true }).click()
  await page.waitForTimeout(700)
  check('the file is gone', await rows(), ['budget.xlsx', 'read.pdf'])
  const undo = page.getByRole('button', { name: 'Undo', exact: true })
  check('undo offers to restore it', await undo.getAttribute('title'), 'Restore one item')
  await undo.click()
  await page.waitForTimeout(900)
  check('the file comes back', await rows(), ['budget.xlsx', 'notes.txt', 'read.pdf'])
})

await scenario('undo a rename with Ctrl+Z', async ({ page, rows, openDocuments }) => {
  await openDocuments()
  await page.locator('[data-path]', { hasText: 'budget.xlsx' }).first().click()
  await page.getByRole('button', { name: 'Rename', exact: true }).click()
  await page.waitForTimeout(500)
  await page.keyboard.press('Control+a')
  await page.keyboard.type('renamed.xlsx')
  await page.keyboard.press('Enter')
  await page.waitForTimeout(800)
  check('the new name sticks', await rows(), ['notes.txt', 'read.pdf', 'renamed.xlsx'])
  await page.keyboard.press('Control+z')
  await page.waitForTimeout(900)
  check('Ctrl+Z puts the old name back', await rows(), ['budget.xlsx', 'notes.txt', 'read.pdf'])
})

await scenario('show hidden files', async ({ page, rows, openDocuments }) => {
  await openDocuments()
  check('dotfiles start hidden', await rows(), ['budget.xlsx', 'notes.txt', 'read.pdf'])
  await page.getByRole('button', { name: 'Settings' }).first().click()
  await page.waitForTimeout(500)
  await page.locator('#setting-hidden').click()
  await page.waitForTimeout(700)
  await openDocuments()
  check('the dotfile appears', await rows(), ['.draft.txt', 'budget.xlsx', 'notes.txt', 'read.pdf'])
})

await scenario('type to filter the folder', async ({ page, rows, openDocuments }) => {
  await openDocuments()
  check('the folder starts unfiltered', await rows(), ['budget.xlsx', 'notes.txt', 'read.pdf'])

  // "ote" sits inside notes.txt without starting it, so a prefix match would miss it.
  await page.keyboard.type('ote')
  await page.waitForTimeout(500)
  check('typing narrows to the substring match', await rows(), ['notes.txt'])
  check('the filter is visible', await page.locator('.file-list__filter-query').innerText(), 'ote')

  check('the top match is selected', await page.locator('[data-path][aria-selected="true"]').count(), 1)

  // The point of selecting it: Enter opens the match without touching the mouse.
  await page.keyboard.press('Backspace')
  await page.keyboard.press('Backspace')
  await page.waitForTimeout(500)
  check('backspace widens the filter', await page.locator('.file-list__filter-query').innerText(), 'o')

  await page.keyboard.press('Escape')
  await page.waitForTimeout(500)
  check('escape restores the folder', await rows(), ['budget.xlsx', 'notes.txt', 'read.pdf'])
  check('the filter bar goes away', await page.locator('.file-list__filter').count(), 0)
})

await scenario('enter opens the filtered match', async ({ page, rows }) => {
  // The Home landing view mounts no file list, so this starts in the home folder.
  await page.locator('.file-sidebar__item', { hasText: 'dave' }).first().click()
  await page.waitForTimeout(700)

  await page.keyboard.type('doc')
  await page.waitForTimeout(500)
  check('the folder narrows to the match', await rows(), ['Documents'])

  await page.keyboard.press('Enter')
  await page.waitForTimeout(700)
  check('enter walked into it', await rows(), ['budget.xlsx', 'notes.txt', 'read.pdf'])
  check('and the filter did not follow', await page.locator('.file-list__filter').count(), 0)
})

await scenario('a filter that matches nothing', async ({ page, rows, openDocuments }) => {
  await openDocuments()
  await page.keyboard.type('zz')
  await page.waitForTimeout(500)

  check('no rows survive', await rows(), [])
  check('the empty state names the filter', await page.locator('.file-list__empty').innerText(), 'Nothing here matches \u201Czz\u201D.')

  // The rows are gone, so nothing in the list holds focus — keys must still land.
  await page.keyboard.press('Backspace')
  await page.waitForTimeout(500)
  check('a keystroke lands with an empty list', await page.locator('.file-list__filter-query').innerText(), 'z')

  await page.keyboard.press('Backspace')
  await page.waitForTimeout(500)
  check('emptying the filter restores the folder', await rows(), ['budget.xlsx', 'notes.txt', 'read.pdf'])
})

await scenario('emptying the recycle bin asks first', async ({ page, rows, openDocuments }) => {
  await openDocuments()
  await page.locator('[data-path]', { hasText: 'notes.txt' }).first().click()
  await page.getByRole('button', { name: 'Delete' }).first().click()
  await page.waitForTimeout(500)

  await page.getByText('Recycle Bin', { exact: false }).first().click()
  await page.waitForTimeout(600)
  check('the trashed file is in the bin', await page.locator('.recycle-list__row').count(), 1)

  await page.getByRole('button', { name: 'Empty Recycle Bin' }).first().click()
  await page.waitForTimeout(300)
  // The webview's own confirm() would block Playwright here instead of rendering.
  check('the app asks with its own dialog', await page.locator('.file-conflict h2').innerText(), 'Empty the Recycle Bin?')

  // A <dialog open> is absolutely positioned by the UA sheet unless overridden.
  const box = await page.locator('.file-conflict').boundingBox()
  const width = page.viewportSize().width
  check('the dialog is centred', Math.abs((box.x + box.width / 2) - width / 2) < 2, true)

  await page.getByRole('button', { name: 'Cancel' }).first().click()
  await page.waitForTimeout(300)
  check('cancel keeps the item', await page.locator('.recycle-list__row').count(), 1)

  await page.getByRole('button', { name: 'Empty Recycle Bin' }).first().click()
  await page.waitForTimeout(300)
  await page.getByRole('button', { name: 'Delete permanently' }).first().click()
  await page.waitForTimeout(500)
  check('confirming empties it', await page.locator('.recycle-list__row').count(), 0)
})

await scenario('a narrow window keeps every command reachable', async ({ page, openDocuments }) => {
  await page.setViewportSize({ width: 600, height: 620 })
  await openDocuments()

  // A tiling WM hands out widths below tauri.conf.json's minWidth, so nothing
  // may sit outside the window: it used to scroll with the scrollbar hidden.
  const shell = await page.evaluate(() => {
    const el = document.querySelector('.app-shell')
    return { client: el.clientWidth, scroll: el.scrollWidth }
  })
  check('the shell does not overflow the window', shell.scroll <= shell.client + 1, true)

  const offscreen = await page.evaluate(() => {
    const out = []
    for (const el of document.querySelectorAll('.file-commands button')) {
      const r = el.getBoundingClientRect()
      if (r.right > window.innerWidth + 1 || r.left < -1) out.push(el.textContent.trim())
    }
    return out
  })
  check('no command sits outside the window', offscreen, [])

  // The search field is sized before the breadcrumb, and used to take its full
  // 254px and leave the trail zero width to render in.
  const trail = await page.evaluate(() => {
    const el = document.querySelector('.app-header__path')
    const last = [...el.querySelectorAll('.app-header__crumb')].pop()
    const r = last.getBoundingClientRect(), box = el.getBoundingClientRect()
    return { width: Math.round(box.width), lastVisible: r.right <= box.right + 1 && r.left >= box.left - 1 }
  })
  check('the breadcrumb still has room', trail.width > 60, true)
  check('the current folder is the crumb on show', trail.lastVisible, true)
})

await scenario('the sidebar collapses to a rail', async ({ page, openDocuments }) => {
  await openDocuments()
  const shell = page.locator('.app-shell')
  const railWidth = async () => Math.round((await page.locator('.file-sidebar').boundingBox()).width)

  check('it starts open', await shell.getAttribute('data-sidebar'), 'open')
  const open = await railWidth()

  await page.getByRole('button', { name: 'Collapse the sidebar' }).click()
  await page.waitForTimeout(300)
  check('collapsing narrows it', await railWidth() < open / 2, true)
  check('the shell knows', await shell.getAttribute('data-sidebar'), 'collapsed')

  // The labels go visually but keep naming their buttons.
  check('Documents is still reachable by name', await page.getByRole('button', { name: 'Documents' }).count() > 0, true)
  const label = page.locator('.file-sidebar__item', { hasText: 'Documents' }).first()
  check('but its label is not on show', Math.round((await label.boundingBox()).width) < 60, true)

  await page.getByRole('button', { name: 'Expand the sidebar' }).click()
  await page.waitForTimeout(300)
  check('expanding restores it', await railWidth(), open)
})

await scenario('sidebar folder tree', async ({ page }) => {
  await page.getByRole('button', { name: 'Expand dave' }).first().click()
  await page.waitForTimeout(600)
  // the drive row shares the tree row styling, so it leads its own children
  const names = await page.locator('.file-sidebar__item--tree').allInnerTexts()
  check('the drive lists its folders beneath it', names, ['dave', 'Documents'])
  await page.locator('.file-sidebar__item--tree', { hasText: 'Documents' }).last().click({ button: 'right' })
  await page.waitForTimeout(400)
  check('rename is offered on a tree folder', await page.getByText('Rename', { exact: true }).count(), 1)
})

await scenario('filesystem navigation', async ({ page, rows }) => {
  await page.getByRole('button', { name: 'Filesystem', exact: true }).click()
  await page.locator('[data-path="/etc"]').waitFor()
  check('filesystem lists root folders', await rows(), ['etc', 'home'])
  check('cannot go above root', await page.getByRole('button', { name: 'Up', exact: true }).isDisabled(), true)
  await page.locator('[data-path="/etc"]').dblclick()
  await page.locator('[data-path="/etc/hosts"]').waitFor()
  check('system folder breadcrumb reaches root', await page.getByRole('navigation', { name: 'Breadcrumb' }).innerText(), 'Files\n/\netc')
  await page.getByRole('button', { name: 'Up', exact: true }).click()
  await page.locator('[data-path="/etc"]').waitFor()
  check('up returns to filesystem', await rows(), ['etc', 'home'])
})

await scenario('disabled standard folder', async ({ page }) => {
  await page.goto(`${URL}?disabled=documents`)
  await page.getByRole('button', { name: 'Downloads', exact: true }).waitFor()
  check('disabled folder shortcut is absent', await page.locator('nav[aria-label="Your files"]').getByRole('button', { name: 'Documents', exact: true }).count(), 0)
})

await scenario('partial search coverage', async ({ page }) => {
  await page.goto(`${URL}?partialSearch=1`)
  const search = page.getByRole('searchbox', { name: 'Search your files', exact: true })
  await search.fill('type:image')
  await page.keyboard.press('Enter')
  const notice = page.getByText('Some files or folders could not be read. These search results may be incomplete.')
  await notice.waitFor()
  check('incomplete coverage is visible', await notice.isVisible(), true)
})

await scenario('AirDrop sharing is explicitly unavailable', async ({ page, openDocuments }) => {
  await openDocuments()
  const share = page.getByRole('button', { name: 'Share', exact: true })
  check('share needs a selection', await share.isDisabled(), true)
  await page.locator('[data-path]', { hasText: 'notes.txt' }).first().click()
  await share.click()
  const dialog = page.getByRole('dialog', { name: 'Share', exact: true })
  await dialog.getByText('Your selected files have not been sent.').waitFor()
  check('selection is shown', await dialog.getByRole('list', { name: 'Items to share' }).innerText(), 'notes.txt')
  check('unsupported is explicit', (await dialog.innerText()).includes('Native iPhone discovery and transfers are not supported yet.'), true)
  check('no send action is offered', await dialog.getByRole('button', { name: 'Send', exact: true }).count(), 0)
  await page.keyboard.press('Escape')
  check('escape closes sharing', await dialog.count(), 0)
  check('focus returns to Share', await share.evaluate((node) => node === document.activeElement), true)
  await page.locator('[data-path]', { hasText: 'read.pdf' }).first().click({ modifiers: ['Control'] })
  await page.locator('[data-path]', { hasText: 'notes.txt' }).first().click({ button: 'right' })
  await page.getByRole('menuitem', { name: 'Share…', exact: true }).click()
  await dialog.getByText('2 items selected').waitFor()
  check('context Share preserves multiple selection', await dialog.getByRole('listitem').count(), 2)
  await dialog.getByRole('button', { name: 'Close', exact: true }).click()
})

await scenario('AirDrop availability errors', async ({ page, openDocuments }) => {
  await page.goto(`${URL}?airdropError=1`)
  await openDocuments()
  await page.locator('[data-path]', { hasText: 'notes.txt' }).first().click()
  await page.getByRole('button', { name: 'Share', exact: true }).click()
  const dialog = page.getByRole('dialog', { name: 'Share', exact: true })
  await dialog.getByRole('alert').waitFor()
  check('availability failure is shown', await dialog.getByRole('alert').innerText(), 'Unable to check AirDrop availability.')
  await dialog.getByRole('button', { name: 'Try again', exact: true }).click()
  await dialog.getByRole('alert').waitFor()
  check('retry remains safe', await dialog.getByRole('button', { name: 'Send', exact: true }).count(), 0)
  await page.keyboard.press('Escape')
})

await scenario('AirDrop delayed response after close', async ({ page, openDocuments }) => {
  await page.goto(`${URL}?airdropSlow=1`)
  await openDocuments()
  await page.locator('[data-path]', { hasText: 'notes.txt' }).first().click()
  await page.getByRole('button', { name: 'Share', exact: true }).click()
  await page.getByText('Checking availability…').waitFor()
  await page.keyboard.press('Escape')
  await page.waitForTimeout(6200)
  check('late response cannot reopen sharing', await page.getByRole('dialog', { name: 'Share', exact: true }).count(), 0)
  await page.getByRole('button', { name: 'Share', exact: true }).click()
  const alert = page.getByRole('alert')
  await alert.waitFor({ timeout: 8000 })
  check('availability timeout is shown', await alert.innerText(), 'Checking AirDrop availability timed out. Try again.')
  await page.waitForTimeout(1200)
  check('late success cannot replace timeout', await alert.isVisible(), true)
  await page.keyboard.press('Escape')
})

await scenario('desktop startup reveals multiple items including hidden files', async ({ page, rows }) => {
  await page.goto(`${URL}?reveal=1`)
  await page.locator('[data-path][aria-selected="true"]').first().waitFor()
  check('requested items sort first', await rows(), ['.draft.txt', 'read.pdf', 'budget.xlsx', 'notes.txt'])
  check('both requested items are selected', await page.locator('[data-path][aria-selected="true"]').count(), 2)
  await page.getByRole('button', { name: 'Restore normal order' }).click()
  await page.waitForTimeout(300)
  check('normal order restores hidden preference', await rows(), ['budget.xlsx', 'notes.txt', 'read.pdf'])
})

await scenario('desktop requests arrive after startup and reuse tabs', async ({ page, openDocuments }) => {
  await openDocuments()
  const send = () => page.evaluate(() => window.__openRequests([{ targets: [
    { folder: '/home/dave/Documents', selection: ['/home/dave/Documents/notes.txt'], properties: null },
    { folder: '/home/dave/Pictures', selection: [], properties: null },
  ], error: null }]))
  await send()
  await page.waitForTimeout(300)
  check('one tab per folder', await page.locator('.app-header__tab').count(), 2)
  await send()
  await page.waitForTimeout(300)
  check('repeated launch reuses existing folders', await page.locator('.app-header__tab').count(), 2)
  await page.locator('.app-header__tab-select', { hasText: 'Documents' }).click()
  await page.locator('[data-path][aria-selected="true"]').first().waitFor()
  check('background tab retains requested selection', await page.locator('[data-path][aria-selected="true"]').getAttribute('data-path'), '/home/dave/Documents/notes.txt')
})

await scenario('desktop errors remain visible on Home', async ({ page }) => {
  await page.goto(`${URL}?openingError=1`)
  await page.getByRole('alert').waitFor()
  check('failed startup is visible', await page.getByRole('alert').locator('p').innerText(), 'This requested folder is unavailable.')
  await page.getByRole('button', { name: 'Dismiss', exact: true }).click()
  check('error can be dismissed', await page.getByRole('alert').count(), 0)
})

await scenario('desktop tab overflow never replaces an existing folder', async ({ page, openDocuments }) => {
  await openDocuments()
  for (let i = 0; i < 11; i++) await page.keyboard.press('Control+t')
  await page.evaluate(() => window.__openRequests([{ targets: [
    { folder: '/home/dave/Pictures', selection: [], properties: null },
  ], error: null }]))
  await page.getByRole('alert').waitFor()
  check('tab limit is respected', await page.locator('.app-header__tab').count(), 12)
  check('overflow explains how to proceed', (await page.getByRole('alert').innerText()).includes('Close a tab'), true)
  check('existing Documents tab preserved', await page.locator('.app-header__tab-select', { hasText: 'Documents' }).count(), 1)
  await page.getByRole('button', { name: 'Dismiss', exact: true }).click()
  await page.locator('.file-sidebar__item', { hasText: 'Pictures' }).first().click({ button: 'right' })
  await page.getByRole('menuitem', { name: 'Open in new tab', exact: true }).click()
  await page.getByRole('alert').waitFor()
  check('sidebar cannot overwrite the active tab at the limit', await page.locator('.app-header__tab--active .app-header__tab-select').innerText(), 'Home')
})

await scenario('closing an earlier tab preserves the current folder', async ({ page, openDocuments }) => {
  await openDocuments()
  await page.evaluate(() => window.__openRequests([{ targets: [
    { folder: '/home/dave/Pictures', selection: [], properties: null },
    { folder: '/home/dave/Downloads', selection: [], properties: null },
  ], error: null }]))
  await page.waitForTimeout(300)
  await page.locator('.app-header__tab-select', { hasText: 'Pictures' }).click()
  await page.locator('.app-header__tab').first().getByRole('button', { name: /Close/ }).click()
  check('Pictures stays active', await page.locator('.app-header__tab--active .app-header__tab-select').innerText(), 'Pictures')
})

await scenario('navigation never offers stale files while the next folder loads', async ({ page, rows, openDocuments }) => {
  await page.goto(`${URL}?slowFolder=1`)
  await openDocuments()
  await page.locator('.file-sidebar__item', { hasText: 'Pictures' }).first().click()
  await page.getByText('Loading Pictures…', { exact: true }).waitFor()
  check('previous folder rows disappear during navigation', await rows(), [])
  check('previous folder commands cannot act during navigation', await page.getByRole('button', { name: 'New folder', exact: true }).count(), 0)
  await page.keyboard.press('Control+Shift+n')
  await page.getByText('This folder is empty.', { exact: true }).waitFor()
  await openDocuments()
  check('new-folder shortcut did not mutate the previous folder', await rows(), ['budget.xlsx', 'notes.txt', 'read.pdf'])
})

await scenario('desktop integration can be enabled and restored', async ({ page }) => {
  await page.getByRole('button', { name: 'Settings', exact: true }).first().click()
  const section = page.getByRole('region', { name: 'Desktop integration', exact: true })
  await section.getByRole('button', { name: 'Make default', exact: true }).click()
  check('enabled status is visible', await section.getByText('Omafil opens your folders', { exact: true }).isVisible(), true)
  check('cold activation status is visible', await section.getByText('Show in folder can launch Omafil automatically.', { exact: true }).isVisible(), true)
  await section.getByRole('button', { name: 'Restore previous setup', exact: true }).click()
  check('restore returns to previous folder handler', await section.getByText('Another app opens your folders', { exact: true }).isVisible(), true)
  check('setup can be enabled again', await section.getByRole('button', { name: 'Make default', exact: true }).isEnabled(), true)
})

await scenario('desktop integration requires an installed application', async ({ page }) => {
  await page.goto(`${URL}?notInstalled=1`)
  await page.getByRole('button', { name: 'Settings', exact: true }).first().click()
  const section = page.getByRole('region', { name: 'Desktop integration', exact: true })
  await section.getByRole('button', { name: 'Make default', exact: true }).waitFor()
  check('development build cannot become default', await section.getByRole('button', { name: 'Make default', exact: true }).isDisabled(), true)
  check('installation requirement is explained', (await section.innerText()).includes('Install this version'), true)
})

await scenario('desktop integration failures stay visible and retryable', async ({ page }) => {
  await page.goto(`${URL}?integrationError=1`)
  await page.getByRole('button', { name: 'Settings', exact: true }).first().click()
  const section = page.getByRole('region', { name: 'Desktop integration', exact: true })
  await section.getByRole('button', { name: 'Make default', exact: true }).click()
  await section.getByRole('alert').waitFor()
  check('failure does not claim default status', await section.getByText('Another app opens your folders', { exact: true }).isVisible(), true)
  await section.getByRole('button', { name: 'Check again', exact: true }).click()
  check('status can be checked again', await section.getByRole('button', { name: 'Make default', exact: true }).isEnabled(), true)
})

await scenario('new folder Enter only saves the name', async ({ page, rows, openDocuments }) => {
  await openDocuments()
  await page.getByRole('button', { name: 'New folder', exact: true }).click()
  const input = page.getByRole('textbox', { name: 'New name', exact: true })
  await input.waitFor()
  check('new folder name has focus', await input.evaluate((el) => el === document.activeElement), true)
  await input.fill('My projects')
  await input.press('Enter')
  await page.waitForTimeout(700)
  check('Enter keeps the parent folder open', await rows(), ['My projects', 'budget.xlsx', 'notes.txt', 'read.pdf'])
  check('rename editor closes', await input.count(), 0)
})

await scenario('second name click renames and double click opens', async ({ page, rows }) => {
  await page.locator('.file-sidebar__item', { hasText: 'dave' }).first().click()
  const label = page.locator('.file-row__label', { hasText: /^Documents$/ })
  await label.click()
  await page.waitForTimeout(700)
  check('first click only selects', await page.getByRole('textbox', { name: 'New name', exact: true }).count(), 0)
  await label.click()
  await page.waitForTimeout(700)
  const input = page.getByRole('textbox', { name: 'New name', exact: true })
  check('second click starts rename', await input.count(), 1)
  if (await input.count()) {
    await input.fill('Do not save')
    await input.press('Escape')
    await page.waitForTimeout(200)
    check('Escape leaves the name unchanged', await label.count(), 1)
  }
  await label.dblclick()
  await page.waitForTimeout(700)
  check('double click still opens the folder', await rows(), ['budget.xlsx', 'notes.txt', 'read.pdf'])
})

await scenario('clipboard files refresh before every paste', async ({ page, openDocuments }) => {
  await openDocuments()
  await page.evaluate(() => window.__setClipboard({ paths: ['/home/dave/.bashrc'], isCut: false }))
  check('paste is available before focus refresh', await page.getByRole('button', { name: 'Paste', exact: true }).isEnabled(), true)
  await page.keyboard.press('Control+v')
  await page.waitForTimeout(200)
  check('external file copy queues a transfer', await page.evaluate(() => window.__clipboardCalls().map((c) => [c.paths, c.isMove, c.destinationPath])), [[['/home/dave/.bashrc'], false, '/home/dave/Documents']])
  await page.evaluate(() => window.__setClipboard({ paths: ['/home/dave/.bashrc'], isCut: true }))
  await page.keyboard.press('Control+v')
  await page.waitForTimeout(200)
  check('same paths changing from copy to cut are respected', await page.evaluate(() => window.__clipboardCalls().at(-1)?.isMove), true)
  await page.evaluate(() => window.__setClipboard(null))
  await page.keyboard.press('Control+v')
  await page.waitForTimeout(200)
  check('empty clipboard never replays old files', await page.evaluate(() => window.__clipboardCalls().length), 2)
  await page.evaluate(() => window.__setClipboard({ error: true }))
  await page.keyboard.press('Control+v')
  await page.waitForTimeout(200)
  check('clipboard failure never replays old files', await page.evaluate(() => window.__clipboardCalls().length), 2)
  check('clipboard failure is visible', await page.getByText('Clipboard service unavailable.', { exact: true }).isVisible(), true)
})

await scenario('clipboard images save separately and can be undone', async ({ page, rows, openDocuments }) => {
  await openDocuments()
  await page.evaluate(() => window.__setClipboard({ image: true }))
  await page.getByRole('button', { name: 'Paste', exact: true }).click()
  await page.waitForTimeout(700)
  check('image is saved in current folder', (await rows()).includes('Clipboard image.png'), true)
  await page.keyboard.press('Control+v')
  await page.waitForTimeout(700)
  check('second image gets a new name', (await rows()).includes('Clipboard image (2).png'), true)
  await page.keyboard.press('Control+z')
  await page.waitForTimeout(700)
  check('undo removes only the second paste', (await rows()).filter((n) => n.startsWith('Clipboard image')), ['Clipboard image.png'])
})

await scenario('copy publication finishes before immediate paste', async ({ page, openDocuments }) => {
  await page.goto(`${URL}?slowClipboard=1`)
  await openDocuments()
  await page.locator('.file-row', { hasText: 'notes.txt' }).click()
  await page.keyboard.press('Control+c')
  await page.locator('.file-sidebar__item', { hasText: 'dave' }).first().click()
  await page.keyboard.press('Control+v')
  await page.waitForTimeout(700)
  check('immediate paste uses the newly copied file', await page.evaluate(() => window.__clipboardCalls().map((c) => [c.paths, c.destinationPath])), [[['/home/dave/Documents/notes.txt'], '/home/dave']])
})

await scenario('font size is readable and persists', async ({ page, openDocuments }) => {
  await page.goto(`${URL}?persistSettings=1`)
  await openDocuments()
  const rowFont = () => page.locator('.file-row').first().evaluate((el) => parseFloat(getComputedStyle(el).fontSize))
  check('default file text is at least 14px', await rowFont() >= 14, true)
  await page.getByRole('button', { name: 'Settings', exact: true }).first().click()
  await page.locator('#setting-font-size').selectOption('125')
  await page.waitForTimeout(600)
  await page.reload()
  await openDocuments()
  check('larger font survives reload', Math.round(await rowFont() * 100) / 100, 18.28)
})

await scenario('network connects with keyboard and keeps URI identity', async ({ page }) => {
  await page.getByRole('button', { name: 'Network & Devices', exact: true }).click()
  await page.getByLabel('Server address', { exact: true }).fill('sftp://test/')
  await page.getByLabel('Remember server address (no credentials)', { exact: true }).check()
  await page.getByLabel('Server address', { exact: true }).press('Enter')
  await page.getByRole('table', { name: 'Remote folder contents' }).waitFor()
  check('connection returns focus to the heading', await page.locator('#network-heading').evaluate((el) => el === document.activeElement), true)
  check('remote file is visible', await page.getByText('report.txt', { exact: true }).isVisible(), true)
  await page.getByRole('button', { name: 'Open folder Documents', exact: true }).click()
  await page.getByText('This folder is empty.', { exact: true }).waitFor()
  await page.getByRole('group', { name: 'Remote folder actions' }).getByRole('button', { name: 'Back', exact: true }).click()
  await page.getByText('report.txt', { exact: true }).waitFor()
  check('listing retains remote scheme', await page.evaluate(() => window.__serviceCalls().filter((c) => c.operation === 'list').every((c) => c.payload.location.kind === 'remote' && c.payload.location.uri.startsWith('sftp:'))), true)
  check('saved server is available for reconnect', await page.getByRole('button', { name: 'sftp://test/', exact: true }).count(), 1)
})

await scenario('network copies in both directions through provider', async ({ page }) => {
  await page.getByRole('button', { name: 'Network & Devices', exact: true }).click()
  await page.getByLabel('Server address', { exact: true }).fill('sftp://test/')
  await page.getByRole('button', { name: 'Connect', exact: true }).click()
  await page.getByLabel('Select report.txt', { exact: true }).check()
  await page.getByRole('button', { name: 'Copy to Downloads', exact: true }).click()
  await page.getByText('1 selected items copied.', { exact: true }).waitFor()
  await page.evaluate(() => window.__setClipboard({ paths: ['/home/dave/Documents/notes.txt'], isCut: false }))
  await page.getByRole('button', { name: 'Paste copied files here', exact: true }).click()
  await page.waitForTimeout(300)
  check('download and upload keep source/destination provider types', await page.evaluate(() => window.__serviceCalls().filter((c) => c.operation === 'copy').map((c) => [c.payload.sources[0].kind, c.payload.destination.kind])), [['remote', 'local'], ['local', 'remote']])
  await page.evaluate(() => window.__setClipboard({ paths: ['/home/dave/Documents/notes.txt'], isCut: true }))
  await page.getByRole('button', { name: 'Paste copied files here', exact: true }).click()
  check('unsupported move is explained', (await page.getByRole('alert').innerText()).includes('Moving to a remote location is not supported'), true)
})

await scenario('dictation starts from the search box', async ({ page }) => {
  const mic = page.getByRole('button', { name: 'Dictate into search', exact: true })
  await mic.waitFor()
  check('the control starts idle', await mic.getAttribute('aria-pressed'), 'false')

  await mic.click()
  const live = page.getByRole('button', { name: 'Stop dictation', exact: true })
  await live.waitFor()
  check('listening is exposed as a pressed state', await live.getAttribute('aria-pressed'), 'true')
  check('dictated text would land in the search box', await page.evaluate(() => document.activeElement?.getAttribute('type')), 'search')
  check('listening is announced', (await page.locator('[role="status"]').allInnerTexts()).some((t) => t.includes('Dictation recording')), true)

  await live.click()
  await page.getByRole('button', { name: 'Dictate into search', exact: true }).waitFor()
  check('stopping returns the control to idle', await mic.getAttribute('aria-pressed'), 'false')

  await page.goto(`${URL}?noDictation=1`)
  check('no voxtype means no control at all', await page.getByRole('button', { name: 'Dictate into search', exact: true }).count(), 0)
})

await scenario('the connect form offers only installed protocols', async ({ page }) => {
  await page.goto(`${URL}?noDevices=1`)
  await page.getByRole('button', { name: 'Network & Devices', exact: true }).click()
  await page.getByText('SFTP, SMB, WebDAV, FTP and FTPS are available', { exact: false }).waitFor()
  check('unencrypted FTP is called out while it is offered', (await page.locator('#server-help').innerText()).includes('FTP sends data without encryption'), true)
  check('phones are offered while a phone backend is present', await page.getByText('unlock your phone', { exact: false }).count(), 1)

  await page.goto(`${URL}?minimalBackends=1&noDevices=1`)
  await page.getByRole('button', { name: 'Network & Devices', exact: true }).click()
  await page.getByText('SFTP is available', { exact: false }).waitFor()
  check('a missing backend is not advertised', (await page.locator('#server-help').innerText()).includes('WebDAV'), false)
  check('the FTP warning goes with FTP', (await page.locator('#server-help').innerText()).includes('without encryption'), false)
  check('the placeholder shows a scheme that works', await page.getByLabel('Server address', { exact: true }).getAttribute('placeholder'), 'sftp://server/path')
  check('no phone backend is explained', (await page.getByText('No phone backend is installed', { exact: false }).count()) > 0, true)

  await page.getByLabel('Server address', { exact: true }).fill('smb://test/share')
  await page.getByRole('button', { name: 'Connect', exact: true }).click()
  check('an uninstalled scheme is refused before connecting', (await page.getByRole('alert').innerText()).includes('cannot connect with smb://'), true)
  check('the refused address never reached the provider', await page.evaluate(() => window.__serviceCalls().some((c) => c.operation === 'connect')), false)
})

await scenario('network connection can be cancelled', async ({ page }) => {
  await page.goto(`${URL}?slowService=1`)
  await page.getByRole('button', { name: 'Network & Devices', exact: true }).click()
  await page.getByLabel('Server address', { exact: true }).fill('sftp://test/')
  await page.getByRole('button', { name: 'Connect', exact: true }).click()
  await page.getByRole('button', { name: 'Cancel operation', exact: true }).click()
  await page.getByRole('alert').waitFor()
  check('cancel is visible and allows retry', await page.getByRole('button', { name: 'Connect', exact: true }).isEnabled(), true)
  check('cancelled connection does not open a folder', await page.getByRole('table', { name: 'Remote folder contents' }).count(), 0)
})

await scenario('indexed content search has coverage filters and reveal', async ({ page }) => {
  await page.goto(`${URL}?limitedIndex=1`)
  await page.getByRole('button', { name: 'Content search', exact: true }).click()
  await page.getByLabel('Words in documents', { exact: true }).fill('quartznectar')
  await page.getByRole('button', { name: 'Search contents', exact: true }).click()
  await page.getByRole('table', { name: 'Indexed search results' }).waitFor()
  check('document without query in its name is shown', await page.getByRole('button', { name: 'notes.txt', exact: true }).count(), 1)
  check('index coverage is explicit', (await page.locator('#index-coverage').innerText()).includes('Only indexed locations'), true)
  check('limited results are explicit', await page.getByText(/first 500 matches/).isVisible(), true)
  await page.getByLabel('Type', { exact: true }).selectOption('images')
  check('type filter narrows results', await page.getByRole('button', { name: 'notes.txt', exact: true }).count(), 0)
  await page.getByLabel('Type', { exact: true }).selectOption('all')
  await page.getByRole('button', { name: 'Show notes.txt in folder', exact: true }).click()
  await page.locator('[data-path="/home/dave/Documents/notes.txt"][aria-selected="true"]').waitFor()
  check('show in folder selects the matched file', await page.locator('[data-path="/home/dave/Documents/notes.txt"][aria-selected="true"]').count(), 1)
})

await scenario('missing index service leaves filename search available', async ({ page }) => {
  await page.goto(`${URL}?missingServices=1`)
  await page.getByRole('button', { name: 'Content search', exact: true }).click()
  await page.getByLabel('Words in documents', { exact: true }).fill('notes')
  await page.getByLabel('Folder', { exact: true }).fill('/home/dave/Documents')
  await page.getByRole('button', { name: 'Search contents', exact: true }).click()
  await page.getByRole('alert').waitFor()
  check('service error is visible', (await page.getByRole('alert').innerText()).includes('LocalSearch is unavailable'), true)
  await page.getByRole('button', { name: 'Search filenames instead', exact: true }).click()
  check('filename search remains available', await page.locator('.service-view').count(), 0)
  await page.waitForTimeout(300)
  check('fallback keeps the requested folder scope', await page.evaluate(() => window.__searchCalls().at(-1)?.path), '/home/dave/Documents')
})

await scenario('keyboard range selection and roving tabs', async ({ page, openDocuments }) => {
  await openDocuments()
  await page.getByRole('option', { name: 'budget.xlsx', exact: true }).click()
  await page.keyboard.press('Shift+ArrowDown')
  check('Shift+Arrow selects the range', await page.locator('.file-row[aria-selected="true"]').count(), 2)
  await page.keyboard.press('Control+Space')
  check('Ctrl+Space toggles the focused item', await page.locator('.file-row[aria-selected="true"]').count(), 1)
  await page.getByRole('button', { name: 'New tab', exact: true }).click()
  const tabs = page.getByRole('tab')
  await tabs.last().focus()
  await page.keyboard.press('ArrowLeft')
  check('arrow key selects the prior tab', await tabs.first().getAttribute('aria-selected'), 'true')
  check('only active tab enters the Tab sequence', await page.locator('[role="tab"][tabindex="0"]').count(), 1)
  await page.keyboard.press('End')
  check('End selects the final tab', await tabs.last().getAttribute('aria-selected'), 'true')
})

await browser.close()
console.log(`\n${checks - failures.length}/${checks} checks passed`)
if (failures.length > 0) {
  console.log(`failed: ${failures.join(', ')}`)
  process.exit(1)
}
