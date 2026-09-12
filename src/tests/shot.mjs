import { createRequire } from 'node:module'
const require = createRequire(import.meta.url)
// Playwright is resolved from PLAYWRIGHT_DIR (a node_modules folder) so the repo carries no browser dependency.
const { chromium } = require(require.resolve('playwright', { paths: [process.env.PLAYWRIGHT_DIR ?? process.cwd()] }))
// usage: PLAYWRIGHT_DIR=<node_modules with playwright> node src/tests/shot.mjs <url> <out.png> [width] [height] [steps...]
// Run 'bunx vite --port 1420' in src/ first; the preview harness lives at /preview.html.
// steps: row:<rowText> | ctrlrow:<rowText> | right:<rowText> | click:<buttonName> | clicklast:<buttonName> | text:<text> | rtext:<text> | fill:<label>=<value> | key:<Key> | wait:<ms>
const [url, out, width = '1200', height = '840', ...steps] = process.argv.slice(2)
const browser = await chromium.launch({ executablePath: '/usr/bin/chromium' })
const page = await browser.newPage({ viewport: { width: Number(width), height: Number(height) } })
page.on('pageerror', (e) => console.log('pageerror', e.message))
page.on('console', (m) => m.type() === 'error' && console.log('console.error', m.text()))
await page.goto(url)
await page.waitForTimeout(600)
for (const step of steps) {
  const [fn, ...rest] = step.split(':'); const arg = rest.join(':')
  if (fn === 'row') await page.locator('[data-path]', { hasText: arg }).first().click()
  else if (fn === 'right') await page.locator('[data-path]', { hasText: arg }).first().click({ button: 'right' })
  else if (fn === 'click') await page.getByRole('button', { name: arg }).first().click()
  else if (fn === 'text') await page.getByText(arg, { exact: false }).first().click()
  else if (fn === 'rtext') await page.getByText(arg, { exact: false }).first().click({ button: 'right' })
  else if (fn === 'ctrlrow') await page.locator('[data-path]', { hasText: arg }).first().click({ modifiers: ['Control'] })
  else if (fn === 'fill') { const [sel, val] = arg.split('='); await page.getByLabel(sel).fill(val) }
  else if (fn === 'clicklast') await page.getByRole('button', { name: arg }).last().click()
  else if (fn === 'key') await page.keyboard.press(arg)
  else if (fn === 'wait') await page.waitForTimeout(Number(arg))
  await page.waitForTimeout(250)
}
await page.screenshot({ path: out })
await browser.close()
