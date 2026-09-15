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

  await page.keyboard.press('Backspace')
  await page.keyboard.press('Backspace')
  await page.waitForTimeout(500)
  check('backspace widens the filter', await page.locator('.file-list__filter-query').innerText(), 'o')

  await page.keyboard.press('Escape')
  await page.waitForTimeout(500)
  check('escape restores the folder', await rows(), ['budget.xlsx', 'notes.txt', 'read.pdf'])
  check('the filter bar goes away', await page.locator('.file-list__filter').count(), 0)
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
