// Layout probe (plan 091 r4): measures the Transakcie and Import screens in a real Chromium against the
// Vite dev server, with a mocked Tauri invoke. Prints one JSON object on stdout.
// Used by src/layout-geometry.test.ts. Prints {"skipped": "<reason>"} when no browser is at hand.
// Browser: ABAKUS_CHROMIUM (path to the executable) or the Playwright cache of this machine.
// Playwright: the node_modules of this repo, ABAKUS_PLAYWRIGHT (path to a package.json beside a node_modules
// with it), or the sibling app ../pdf-editor of this machine.
import { createRequire } from 'node:module'
import { existsSync, readdirSync } from 'node:fs'
import { join } from 'node:path'
import { createServer } from 'vite'

function findBrowser() {
  if (process.env.ABAKUS_CHROMIUM) return existsSync(process.env.ABAKUS_CHROMIUM) ? process.env.ABAKUS_CHROMIUM : null
  const cache = join(process.env.LOCALAPPDATA ?? '', 'ms-playwright')
  if (!existsSync(cache)) return null
  for (const dir of readdirSync(cache).filter((d) => d.startsWith('chromium_headless_shell')).sort().reverse()) {
    const exe = join(cache, dir, 'chrome-headless-shell-win64', 'chrome-headless-shell.exe')
    if (existsSync(exe)) return exe
  }
  return null
}

function loadPlaywright() {
  for (const base of [join(process.cwd(), 'package.json'), process.env.ABAKUS_PLAYWRIGHT, join(process.cwd(), '..', 'pdf-editor', 'package.json')]) {
    if (!base) continue
    try { return createRequire(base)('playwright') } catch { /* try the next place */ }
  }
  return null
}

const mock = (scenario) => {
  const cats = [
    { id: 1, parent_id: null, name: scenario === 'long' ? 'Veľmi dlhý názov kategórie '.repeat(5).trim() : 'Potraviny', kind: 'expense', sort: 1, system: false, archived: false },
    { id: 2, parent_id: null, name: 'Bývanie', kind: 'expense', sort: 2, system: false, archived: false },
    { id: 3, parent_id: null, name: 'Doprava', kind: 'expense', sort: 3, system: false, archived: false },
    { id: 4, parent_id: null, name: 'Mzda', kind: 'income', sort: 4, system: false, archived: false },
  ]
  const merchants = ['LIDL BRATISLAVA', 'TESCO EXPRESS', 'SLOVNAFT', 'NÁJOM 10/2026', 'Zamestnávateľ s.r.o.', 'BOLT RIDE', 'KAUFLAND']
  const tx = scenario === 'empty' ? [] : Array.from({ length: 24 }, (_, i) => {
    const income = i % 8 === 3
    const cat = income ? cats[3] : cats[i % 3]
    const status = i % 11 === 10 ? 'transfer' : i % 5 === 0 ? 'unassigned' : i % 7 === 0 ? 'suggested' : 'confirmed'
    return {
      id: i + 1, account_id: 1, account_kind: 'personal', statement_number: 10, posted_date: `2026-10-${String(1 + (i % 28)).padStart(2, '0')}`,
      tx_date: `2026-10-${String(1 + (i % 28)).padStart(2, '0')}`, kind: income ? 'transfer_in' : 'card',
      amount_cents: income ? 185000 : -(1250 + i * 733), orig_amount_cents: null, orig_currency: null,
      merchant_raw: status === 'transfer' ? 'Prevod' : merchants[i % merchants.length], place: 'Bratislava', counterparty_name: null, counterparty_iban: null,
      category_id: status === 'unassigned' ? null : cat.id, category_name: status === 'unassigned' ? null : cat.name, parent_name: null,
      status, source: 'rule', raw_block: '', note: '',
    }
  })
  const accounts = [{ id: 1, iban: 'SK3112000000198742637541', kind: 'personal', label: 'Bežný účet', has_password: false }]
  const handlers = {
    list_accounts: accounts, list_categories: cats, list_rules: [], list_transactions: tx,
    statement_history: [0, 1, 2].map((i) => ({ statement_id: i + 1, account_id: 1, account_label: 'Bežný účet (demo)', account_kind: 'personal', number: 10 - i, period_start: '2026-10-01', period_end: '2026-10-31', opening_cents: 0, closing_cents: 10000, transaction_count: 48, total_cents: -65000, checksum: i === 0 ? { status: 'off_by', off_by: 150 } : { status: 'ok' } })), statement_review: { checksum: { status: 'ok' }, parser_warnings: [], unassigned_count: 2, suggested_count: 1, status: 'needs_attention' }, recent_statements: [], bad_checksums: [], get_check_updates: false, net_log: [], net_audit_failures: [],
    data_dir: 'C:\\Users\\demo\\AppData\\Roaming\\Abakus',
  }
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: 'main' }, currentWebview: { windowLabel: 'main', label: 'main' } },
    transformCallback: () => 1, unregisterCallback: () => {},
    invoke: async (cmd) => (cmd in handlers ? handlers[cmd] : null),
  }
}

async function openRegister(browser, width, scenario) {
  const ctx = await browser.newContext({ viewport: { width, height: width === 1024 ? 680 : width === 1280 ? 820 : 900 } })
  await ctx.addInitScript(mock, scenario)
  await ctx.addInitScript(() => { try { localStorage.setItem('kaliber-theme', 'light'); localStorage.setItem('abakus.period', JSON.stringify({ kind: 'all' })) } catch { /* private mode */ } })
  const page = await ctx.newPage()
  return { ctx, page }
}

async function goToRegister(page, url, scenario) {
  await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 170000 })
  await page.locator('nav[aria-label="Sekcie"]').getByText('Transakcie', { exact: true }).first().click({ timeout: 30000 })
  await page.waitForSelector(scenario === 'empty' ? 'text=Za toto obdobie' : 'tbody tr[data-row-id]', { timeout: 30000 })
  await page.waitForTimeout(500)
}

// The click area of a checkbox: its wrapping label when it has one, else the box itself.
const size = (input) => input.evaluate((el) => { const r = (el.closest('label') ?? el).getBoundingClientRect(); return { width: Math.round(r.width * 10) / 10, height: Math.round(r.height * 10) / 10 } })

async function registerFit(page) {
  return page.evaluate(() => {
    const region = document.querySelector('[aria-label="Transakcie"][role="region"]')
    const table = region.querySelector('table')
    const amount = table.querySelector('tbody tr[data-row-id] .ab-money').getBoundingClientRect()
    const bounds = region.getBoundingClientRect()
    const note = table.querySelector('textarea')?.getBoundingClientRect()
    return { overflow: region.scrollWidth - region.clientWidth, scrollLeft: region.scrollLeft,
      amountVisible: amount.left >= bounds.left && amount.right <= bounds.right,
      detailVisible: !note || (note.left >= bounds.left && note.right <= bounds.right),
      pageOverflow: document.documentElement.scrollWidth - document.documentElement.clientWidth }
  })
}

async function measureOpenDetail(page, width) {
  const closed = await registerFit(page)
  await page.locator('tbody tr[data-row-id="1"] input[type=checkbox]').check()
  await page.locator('tbody tr[data-row-id="1"]').getByRole('button', { name: /detail/i }).click()
  await page.locator('textarea').waitFor()
  const open = await registerFit(page)
  if (process.env.ABAKUS_SHOT_DIR) await page.screenshot({ path: join(process.env.ABAKUS_SHOT_DIR, `detail-${width}.png`), fullPage: true })
  return { closed, open }
}

async function importFit(browser, url, width) {
  const { ctx, page } = await openRegister(browser, width, 'base')
  try {
    await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 170000 })
    await page.locator('nav[aria-label="Sekcie"]').getByRole('button', { name: 'Import', exact: true }).click()
    const card = page.getByRole('region', { name: 'Všetky výpisy', exact: true })
    await card.getByText('Kontrolný súčet nesedí o 1,50 €', { exact: true }).waitFor()
    await card.getByText('Vyžaduje pozornosť', { exact: true }).first().waitFor()
    return await card.evaluate(el => {
      const bounds = el.getBoundingClientRect()
      const buttons = [...el.querySelectorAll('button[aria-label^="Zmazať výpis"]')]
      return { rowCount: buttons.length, deleteInside: buttons.map(button => {
        const rect = button.getBoundingClientRect()
        return rect.left >= bounds.left && rect.right <= bounds.right
      }) }
    })
  } finally { await ctx.close() }
}

async function importSizes(browser, url) {
  const sizes = {}
  for (const width of [1024, 1280]) sizes[width] = await importFit(browser, url, width)
  return sizes
}

async function main() {
  const playwright = loadPlaywright()
  const executablePath = findBrowser()
  if (!playwright || !executablePath) {
    console.log(JSON.stringify({ skipped: playwright ? 'no Chromium found (set ABAKUS_CHROMIUM)' : 'playwright not found (set ABAKUS_PLAYWRIGHT)' }))
    return
  }
  const server = await createServer({ logLevel: 'silent', server: { port: 1431, strictPort: false, host: '127.0.0.1' } })
  await server.listen()
  const url = `http://127.0.0.1:${server.httpServer.address().port}/`
  const browser = await playwright.chromium.launch({ headless: true, executablePath })
  const out = { firstRowTop: {}, overflowX: {}, hit: {}, minimum: {}, imports: {} }
  try {
    for (const width of [1440, 1280, 1024]) {
      const { ctx, page } = await openRegister(browser, width, 'base')
      await goToRegister(page, url, 'base')
      out.firstRowTop[width] = await page.evaluate(() => Math.round(document.querySelector('tbody tr[data-row-id]').getBoundingClientRect().top + window.scrollY))
      if (process.env.ABAKUS_SHOT_DIR) await page.screenshot({ path: join(process.env.ABAKUS_SHOT_DIR, `register-${width}.png`), fullPage: true })
      // The search field shows its whole placeholder: text width against the room inside the field.
      out.search = out.search ?? {}
      out.search[width] = await page.evaluate(() => {
        const input = document.getElementById('f-q')
        const style = getComputedStyle(input)
        const context = document.createElement('canvas').getContext('2d')
        context.font = `${style.fontStyle} ${style.fontWeight} ${style.fontSize} ${style.fontFamily}`
        const room = input.clientWidth - parseFloat(style.paddingLeft) - parseFloat(style.paddingRight)
        return { need: Math.ceil(context.measureText(input.placeholder).width), room: Math.floor(room) }
      })
      // Where each block above the register starts and ends: the report shows what takes the room.
      out.layers = out.layers ?? {}
      out.layers[width] = await page.evaluate(() => Object.fromEntries(['.ab-head', '.k-ink-row', '.ab-filters', '.ab-cap', '.ab-table-scroll', 'thead'].map((sel) => {
        const r = document.querySelector(sel)?.getBoundingClientRect()
        return [sel, r ? [Math.round(r.top + window.scrollY), Math.round(r.bottom + window.scrollY)] : null]
      })))
      if (width === 1024 || width === 1280) {
        out.minimum[width] = await measureOpenDetail(page, width)
      }
      out.overflowX[width] = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)
      await ctx.close()
    }
    out.imports = await importSizes(browser, url)
    for (const scenario of ['long', 'empty']) {
      const { ctx, page } = await openRegister(browser, 1440, scenario)
      await goToRegister(page, url, scenario)
      out.overflowX[scenario] = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)
      if (scenario === 'empty') out.emptyTop = await page.evaluate(() => Math.round(document.querySelector('tbody tr').getBoundingClientRect().top + window.scrollY))
      await ctx.close()
    }
    const { ctx, page } = await openRegister(browser, 390, 'base')
    await goToRegister(page, url, 'base')
    const rowBox = page.locator('tbody tr[data-row-id="1"] input[type=checkbox]').first()
    await rowBox.scrollIntoViewIfNeeded()
    out.hit.rowCheckbox = await size(rowBox)
    const transfer = page.locator('tbody tr[data-row-id="11"] input[type=checkbox]').first()
    out.hit.transferDisabled = await transfer.isDisabled()
    await transfer.scrollIntoViewIfNeeded()
    out.hit.transferCheckbox = await size(transfer)
    const similar = page.locator('tbody label', { hasText: 'Potvrdiť aj podobné' }).first().locator('input')
    await similar.scrollIntoViewIfNeeded()
    out.hit.similar = await size(similar)
    out.overflowX[390] = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth)
    await ctx.close()
  } finally {
    await browser.close()
    await server.close()
  }
  console.log(JSON.stringify(out))
}

main().then(() => process.exit(0), (error) => { console.error(error); process.exit(1) })
