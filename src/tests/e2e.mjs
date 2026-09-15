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

await browser.close()
console.log(`\n${checks - failures.length}/${checks} checks passed`)
if (failures.length > 0) {
  console.log(`failed: ${failures.join(', ')}`)
  process.exit(1)
}
