import { spawn, spawnSync } from 'node:child_process'
import crypto from 'node:crypto'
import fs from 'node:fs'
import path from 'node:path'
import { pathToFileURL } from 'node:url'

const ROOT = path.resolve(import.meta.dirname, '..')
const READY_PATH = path.join(ROOT, 'acceptance', 'pdf-ready.json')
const RESULTS_DIR = path.join(ROOT, 'acceptance', 'pdf-native-results')
const PAGE_RESULTS_DIR = path.join(ROOT, 'acceptance', 'pdf-page-results')
const PLAYWRIGHT_PATH = 'A:/projects-vault/apps/pdf-editor/node_modules/playwright/index.mjs'
const REQUIRED_READY_FIELDS = ['exe_path', 'binary_sha256', 'pid', 'cdp_endpoint', 'data_dir', 'oracle_path']

function fail(message) {
  throw new Error(message)
}

function assert(condition, message) {
  if (!condition) fail(message)
}

function assertEqual(actual, expected, label) {
  const left = JSON.stringify(actual)
  const right = JSON.stringify(expected)
  if (left !== right) fail(`${label}: expected ${right}, got ${left}`)
}

function readJson(filePath) {
  return JSON.parse(fs.readFileSync(filePath, 'utf8'))
}

function sha256(filePath) {
  return crypto.createHash('sha256').update(fs.readFileSync(filePath)).digest('hex')
}

function assertInside(parent, child, label) {
  const relative = path.relative(path.resolve(parent), path.resolve(child))
  assert(relative && !relative.startsWith('..') && !path.isAbsolute(relative), `${label} must be inside ${parent}: ${child}`)
}

function writeNewJson(filePath, value) {
  assertInside(RESULTS_DIR, filePath, 'result')
  fs.mkdirSync(path.dirname(filePath), { recursive: true })
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, { flag: 'wx' })
}

function readReady() {
  assert(fs.existsSync(READY_PATH), `missing final readiness manifest: ${READY_PATH}`)
  const ready = readJson(READY_PATH)
  for (const field of REQUIRED_READY_FIELDS) assert(ready[field] !== undefined && ready[field] !== null, `pdf-ready.json missing ${field}`)
  assert(Number.isSafeInteger(ready.pid) && ready.pid > 0, `invalid PID: ${ready.pid}`)
  assert(fs.statSync(ready.exe_path).isFile(), `executable is missing: ${ready.exe_path}`)
  assertEqual(sha256(ready.exe_path), ready.binary_sha256.toLowerCase(), 'ready executable SHA256')
  assert(fs.statSync(ready.oracle_path).isFile(), `oracle is missing: ${ready.oracle_path}`)
  const oracle = readJson(ready.oracle_path)
  assertEqual(path.resolve(oracle.data_dir), path.resolve(ready.data_dir), 'oracle and ready data_dir')
  assertInside(path.join(ROOT, 'acceptance'), ready.data_dir, 'synthetic data_dir')
  assertInside(path.join(ROOT, 'acceptance'), oracle.db_path, 'synthetic database')
  try {
    process.kill(ready.pid, 0)
  } catch (error) {
    fail(`ready PID ${ready.pid} is not running: ${error}`)
  }
  return { ready, oracle }
}

async function invoke(page, command, args = {}) {
  return page.evaluate(
    ([nativeCommand, nativeArgs]) => window.__TAURI_INTERNALS__.invoke(nativeCommand, nativeArgs),
    [command, args],
  )
}

async function invokeError(page, command, args = {}) {
  try {
    await invoke(page, command, args)
  } catch (error) {
    return String(error)
  }
  fail(`${command} unexpectedly succeeded`)
}

async function attach() {
  const { ready, oracle } = readReady()
  const playwright = await import(pathToFileURL(PLAYWRIGHT_PATH).href)
  const browser = await playwright.chromium.connectOverCDP(ready.cdp_endpoint)
  const pages = browser.contexts().flatMap((context) => context.pages())
  const page = pages.find((candidate) => candidate.url().startsWith('tauri://') || candidate.url().includes('tauri.localhost'))
  assert(page, `CDP has no Tauri page. URLs: ${pages.map((candidate) => candidate.url()).join(', ')}`)
  assert(!/^https?:\/\/(localhost|127\.0\.0\.1)/.test(page.url()), `refusing development page: ${page.url()}`)
  await page.waitForFunction(() => typeof window.__TAURI_INTERNALS__?.invoke === 'function')

  // This must remain the first native call. It prevents a mutation against a
  // wrong profile even when a stale CDP endpoint is supplied.
  const actualDataDir = await invoke(page, 'data_dir')
  assertEqual(path.resolve(actualDataDir), path.resolve(ready.data_dir), 'genuine native data_dir')
  return { browser, page, ready, oracle, actualDataDir }
}

const PY_DB_STATE = String.raw`
import hashlib, json, sqlite3, sys
db = sys.argv[1]
tables = ["accounts","statements","categories","rules","transactions","settings","rule_sources","recurring_decisions","recurring_members"]
con = sqlite3.connect(db)
def table_state(table):
    columns = [row[1] for row in con.execute(f'PRAGMA table_info("{table}")')]
    order = ", ".join(f'"{column}"' for column in columns)
    rows = con.execute(f'SELECT * FROM "{table}" ORDER BY {order}').fetchall()
    payload = json.dumps(rows, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    return {"count": len(rows), "sha256": hashlib.sha256(payload).hexdigest()}
result = {
    "integrity": con.execute("PRAGMA integrity_check").fetchone()[0],
    "foreign_key_errors": con.execute("PRAGMA foreign_key_check").fetchall(),
    "settings": dict(con.execute("SELECT key,value FROM settings ORDER BY key")),
    "tables": {table: table_state(table) for table in tables},
}
con.close()
print(json.dumps(result, ensure_ascii=False))
`

function databaseState(dbPath) {
  const result = spawnSync('py', ['-c', PY_DB_STATE, dbPath], { encoding: 'utf8', windowsHide: true })
  if (result.status !== 0) fail(`independent sqlite inspection failed: ${result.stderr || result.stdout}`)
  return JSON.parse(result.stdout)
}

function assertInitialDatabase(oracle) {
  const state = databaseState(oracle.db_path)
  assertEqual(state.integrity, 'ok', 'SQLite integrity_check')
  assertEqual(state.foreign_key_errors, [], 'SQLite foreign_key_check')
  assertEqual(state.settings.schema_version, '4', 'schema version')
  for (const [table, expected] of Object.entries(oracle.initial_state)) {
    assertEqual(state.tables[table], expected, `initial table ${table}`)
  }
  return state
}

function assertPreview(preview, family, label) {
  assertEqual(preview.range, family.range, `${label} range`)
  assertEqual(preview.transaction_count, family.transaction_count, `${label} transaction count`)
  assert(Number.isSafeInteger(preview.transaction_count), `${label} count is not a safe integer`)
  assert(Array.isArray(preview.accounts), `${label} accounts is not an array`)
  assert(typeof preview.captured_at === 'string' && /[+-]\d\d:\d\d$/.test(preview.captured_at), `${label} captured_at lacks a numeric offset: ${preview.captured_at}`)
}

async function previewFamilies(page, oracle) {
  const previews = {}
  for (const [name, family] of Object.entries(oracle.families)) {
    const preview = await invoke(page, 'preview_pdf_report', { request: family.request })
    assertPreview(preview, family, name)
    previews[name] = preview
  }
  return previews
}

async function assertRecurringOracle(page, oracle) {
  const query = { from: '2026-01-01', to: '2026-04-30', account_kind: 'personal', today: '2026-04-15' }
  const overview = await invoke(page, 'recurring_overview', { query })
  const expected = oracle.recurring.anchor_candidate
  const row = overview.rows.find((candidate) => candidate.name === expected.name)
  assert(row, `recurring overview lacks ${expected.name}`)
  assertEqual(row.cadence, expected.cadence, 'R01 inferred cadence')
  assertEqual(row.anchor_date, expected.anchor_date, 'R01 anchor')
  assertEqual(row.next_due, expected.next_due, 'R01 next due')
  assertEqual(row.state, expected.state_at_2026_04_15, 'R01 state')
  assertEqual(row.decision, 'estimate', 'R01 remains an estimate')
  return { query, row, overview }
}

async function inspectMode() {
  const { browser, page, ready, oracle, actualDataDir } = await attach()
  try {
    const database = assertInitialDatabase(oracle)
    const accounts = await invoke(page, 'list_accounts')
    assertEqual(
      accounts.map(({ id, kind, label }) => ({ id, kind, label })),
      oracle.expected_accounts,
      'native account list',
    )
    const previews = await previewFamilies(page, oracle)
    const recurring = await assertRecurringOracle(page, oracle)
    const malformedReport = await invokeError(page, 'preview_pdf_report', {
      request: { ...oracle.families.month.request, hidden_table_filter: 'must fail' },
    })
    assert(/unknown|field|deserialize|hidden_table_filter/i.test(malformedReport), `unknown report field error is unclear: ${malformedReport}`)
    const future = await invokeError(page, 'preview_pdf_report', {
      request: { period: { kind: 'month', month: '2026-10' }, scope: { kind: 'account', account_id: 1 } },
    })
    assert(/future|budúc|začína/i.test(future), `future period error is unclear: ${future}`)

    const result = {
      mode: 'inspect',
      commit: ready.commit,
      executable: { path: ready.exe_path, sha256: ready.binary_sha256, pid: ready.pid },
      data_dir: actualDataDir,
      page: { url: page.url(), title: await page.title(), invoke_type: await page.evaluate(() => typeof window.__TAURI_INTERNALS__?.invoke) },
      database,
      accounts,
      previews,
      recurring: { query: recurring.query, row: recurring.row },
      malformed_report_error: malformedReport,
      future_period_error: future,
    }
    writeNewJson(path.join(RESULTS_DIR, 'inspect.json'), result)
    console.log(JSON.stringify({ ok: true, mode: 'inspect', data_dir: actualDataDir }))
  } finally {
    await browser.close()
  }
}

const DIALOG_AUTOMATION = String.raw`
Add-Type -AssemblyName UIAutomationClient
$targetPid = [int]$env:ABAKUS_ACCEPT_PID
$action = $env:ABAKUS_ACCEPT_ACTION
$targetPath = $env:ABAKUS_ACCEPT_PATH
$root = [System.Windows.Automation.AutomationElement]::RootElement
$pidCondition = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty, $targetPid)
$deadline = [DateTime]::UtcNow.AddSeconds(25)
$dialog = $null
$edit = $null
while ([DateTime]::UtcNow -lt $deadline -and $null -eq $edit) {
  $windows = $root.FindAll([System.Windows.Automation.TreeScope]::Children, $pidCondition)
  foreach ($window in $windows) {
    $idCondition = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::AutomationIdProperty, '1001')
    $candidate = $window.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $idCondition)
    if ($null -ne $candidate) { $dialog = $window; $edit = $candidate; break }
  }
  if ($null -eq $edit) { [System.Threading.ManualResetEventSlim]::new($false).Wait(100) }
}
if ($null -eq $edit) { throw "Save dialog for PID $targetPid was not found" }
if ($action -eq 'cancel') {
  $cancelCondition = New-Object System.Windows.Automation.OrCondition(
    (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::AutomationIdProperty, '2')),
    (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty, 'Zrušiť'))
  )
  $cancel = $dialog.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $cancelCondition)
  if ($null -eq $cancel) { throw 'Cancel button was not found' }
  $cancel.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
  '{"action":"cancel","ok":true}'
  exit 0
}
$edit.SetFocus()
$edit.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern).SetValue($targetPath)
$saveCondition = New-Object System.Windows.Automation.OrCondition(
  (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::AutomationIdProperty, '1')),
  (New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty, 'Uložiť'))
)
$save = $dialog.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $saveCondition)
if ($null -eq $save) { throw 'Save button was not found' }
$save.SetFocus()
$save.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
if ($action -eq 'occupied') {
  $confirm = $null
  while ([DateTime]::UtcNow -lt $deadline -and $null -eq $confirm) {
    $confirmCondition = New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::AutomationIdProperty, 'CommandButton_6')
    $confirm = $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $confirmCondition)
    if ($null -eq $confirm) { [System.Threading.ManualResetEventSlim]::new($false).Wait(100) }
  }
  if ($null -eq $confirm) { throw 'Occupied-target confirmation button was not found' }
  $confirm.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern).Invoke()
}
'{"action":"save","ok":true}'
`

function automateDialog(pid, action, targetPath = '') {
  return new Promise((resolve, reject) => {
    const child = spawn('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', '-'], {
      env: { ...process.env, ABAKUS_ACCEPT_PID: String(pid), ABAKUS_ACCEPT_ACTION: action, ABAKUS_ACCEPT_PATH: targetPath },
      windowsHide: true,
      stdio: ['pipe', 'pipe', 'pipe'],
    })
    let stdout = ''
    let stderr = ''
    child.stdout.on('data', (chunk) => { stdout += chunk })
    child.stderr.on('data', (chunk) => { stderr += chunk })
    child.on('error', reject)
    child.on('close', (code) => {
      if (code !== 0) reject(new Error(`dialog automation failed (${code}): ${stderr || stdout}`))
      else resolve({ action, stdout: stdout.trim() })
    })
    child.stdin.end(DIALOG_AUTOMATION)
  })
}

async function openExportDialog(page) {
  await page.getByRole('button', { name: 'Exportovať PDF', exact: true }).click()
  const dialog = page.getByRole('dialog', { name: 'Exportovať PDF' })
  await dialog.waitFor({ state: 'visible' })
  return dialog
}

async function chooseFamily(dialog, name, family) {
  const period = family.request.period
  await dialog.getByLabel('Obdobie').selectOption(period.kind)
  if (period.kind === 'month') await dialog.getByLabel('Mesiac').fill(period.month)
  if (period.kind === 'six_months') await dialog.getByLabel('Koncový mesiac').fill(period.ending_month)
  if (period.kind === 'year') await dialog.getByLabel('Rok').fill(String(period.year))
  const scope = family.request.scope
  const scopeValue = scope.kind === 'account' ? `account:${scope.account_id}` : scope.kind === 'kind' ? scope.account_kind : 'all'
  await dialog.getByLabel('Účty').selectOption(scopeValue)
  const preview = dialog.getByRole('region', { name: 'Náhľad PDF reportu' })
  await preview.waitFor({ state: 'visible' })
  await preview.getByText(String(family.transaction_count), { exact: true }).waitFor()
  return { name, preview }
}

async function saveFamilyThroughUi(page, dialog, ready, name, family, targetPath) {
  assert(!fs.existsSync(targetPath), `${name} target already exists: ${targetPath}`)
  await chooseFamily(dialog, name, family)
  const automation = automateDialog(ready.pid, 'save', targetPath)
  await dialog.getByRole('button', { name: 'Uložiť PDF', exact: true }).click()
  await automation
  const status = dialog.getByRole('status').filter({ hasText: 'PDF uložené:' })
  await status.waitFor({ timeout: 30000 })
  assert(fs.statSync(targetPath).isFile(), `${name} PDF was not created`)
  const statusText = await status.innerText()
  assert(statusText.includes(`Transakcie zachytené pri uložení: ${family.transaction_count}`), `${name} success uses a stale count: ${statusText}`)
  return { path: targetPath, sha256: sha256(targetPath), bytes: fs.statSync(targetPath).size, status: statusText }
}

async function saveDirect(page, name, family, targetPath) {
  assert(!fs.existsSync(targetPath), `${name} target already exists: ${targetPath}`)
  const outcome = await invoke(page, 'export_pdf_report', { request: family.request, path: targetPath })
  assertEqual(path.resolve(outcome.path), path.resolve(targetPath), `${name} outcome path`)
  assertEqual(outcome.report.transaction_count, family.transaction_count, `${name} captured count`)
  assertEqual(outcome.bytes, fs.statSync(targetPath).size, `${name} outcome bytes`)
  return { outcome, path: targetPath, sha256: sha256(targetPath), bytes: fs.statSync(targetPath).size }
}

async function nativeCancel(page, dialog, ready, family) {
  await chooseFamily(dialog, 'cancel', family)
  const existingSuccess = await dialog.getByRole('status').filter({ hasText: 'PDF uložené:' }).count()
  const automation = automateDialog(ready.pid, 'cancel')
  await dialog.getByRole('button', { name: 'Uložiť PDF', exact: true }).click()
  await automation
  await dialog.getByRole('button', { name: 'Uložiť PDF', exact: true }).waitFor({ state: 'visible' })
  assertEqual(await dialog.getByRole('status').filter({ hasText: 'PDF uložené:' }).count(), existingSuccess, 'cancel adds no success state')
  return { cancelled: true, success_count_before: existingSuccess }
}

async function occupiedUi(page, dialog, ready, family, targetPath) {
  fs.writeFileSync(targetPath, 'PDF-OCCUPIED-SENTINEL\n', { flag: 'wx' })
  const before = sha256(targetPath)
  await chooseFamily(dialog, 'occupied', family)
  const automation = automateDialog(ready.pid, 'occupied', targetPath)
  await dialog.getByRole('button', { name: 'Uložiť PDF', exact: true }).click()
  await automation
  const alert = dialog.getByRole('alert')
  await alert.waitFor({ timeout: 30000 })
  assertEqual(sha256(targetPath), before, 'occupied UI target changed')
  assertEqual(await dialog.getByRole('status').filter({ hasText: 'PDF uložené:' }).count(), 0, 'occupied UI target shows success')
  return { path: targetPath, sha256: before, alert: await alert.innerText() }
}

async function fileSafetyCanaries(page, oracle) {
  const request = oracle.families.empty.request
  const sourceBefore = sha256(oracle.db_path)
  const occupied = path.join(RESULTS_DIR, 'direct-occupied.pdf')
  fs.writeFileSync(occupied, 'DIRECT-OCCUPIED-SENTINEL\n', { flag: 'wx' })
  const occupiedBefore = sha256(occupied)
  const existingError = await invokeError(page, 'export_pdf_report', { request, path: occupied })
  assertEqual(sha256(occupied), occupiedBefore, 'direct occupied target changed')

  const hardlink = path.join(RESULTS_DIR, 'database-hardlink.pdf')
  fs.linkSync(oracle.db_path, hardlink)
  const hardlinkError = await invokeError(page, 'export_pdf_report', { request, path: hardlink })
  assertEqual(sha256(oracle.db_path), sourceBefore, 'source database changed through hardlink target')

  let symlink = { supported: false }
  const symlinkPath = path.join(RESULTS_DIR, 'database-symlink.pdf')
  try {
    fs.symlinkSync(oracle.db_path, symlinkPath, 'file')
    const error = await invokeError(page, 'export_pdf_report', { request, path: symlinkPath })
    assertEqual(sha256(oracle.db_path), sourceBefore, 'source database changed through symlink target')
    symlink = { supported: true, error }
  } catch (error) {
    symlink = { supported: false, reason: String(error) }
  }

  const directoryPath = path.join(RESULTS_DIR, 'directory.pdf')
  fs.mkdirSync(directoryPath)
  const errors = {
    occupied: existingError,
    hardlink: hardlinkError,
    directory: await invokeError(page, 'export_pdf_report', { request, path: directoryPath }),
    missing_parent: await invokeError(page, 'export_pdf_report', { request, path: path.join(RESULTS_DIR, 'missing-parent', 'report.pdf') }),
    alternate_stream: await invokeError(page, 'export_pdf_report', { request, path: `${occupied}:stream.pdf` }),
    device: await invokeError(page, 'export_pdf_report', { request, path: '\\\\.\\NUL.pdf' }),
    database_path: await invokeError(page, 'export_pdf_report', { request, path: oracle.db_path }),
  }

  const raceTarget = path.join(RESULTS_DIR, 'two-writers.pdf')
  const race = await Promise.allSettled([
    invoke(page, 'export_pdf_report', { request, path: raceTarget }),
    invoke(page, 'export_pdf_report', { request, path: raceTarget }),
  ])
  assertEqual(race.filter((item) => item.status === 'fulfilled').length, 1, 'two-writer success count')
  assertEqual(race.filter((item) => item.status === 'rejected').length, 1, 'two-writer rejection count')
  assert(fs.statSync(raceTarget).isFile(), 'two-writer winning PDF is missing')
  assertEqual(sha256(oracle.db_path), sourceBefore, 'file safety canaries changed source database')
  return { source_sha256: sourceBefore, errors, symlink, race: race.map((item) => item.status), winner_sha256: sha256(raceTarget) }
}

async function openScreen(page, name) {
  await page.getByRole('navigation').getByRole('button', { name, exact: true }).click()
  await page.getByRole('heading', { name, exact: true }).waitFor()
}

async function showTransaction(page, transactionId, merchant) {
  await openScreen(page, 'Transakcie')
  await page.getByRole('group', { name: 'Obdobie' }).getByRole('button', { name: 'Všetko', exact: true }).click()
  await page.getByLabel('Hľadať obchodníka alebo poznámku').fill(merchant)
  const detail = page.getByRole('button', { name: `Detail transakcie ${transactionId}`, exact: true })
  await detail.waitFor()
  if ((await detail.getAttribute('aria-expanded')) !== 'true') await detail.click()
}

async function editRecurringThroughUi(page, transactionId, merchant, action) {
  await showTransaction(page, transactionId, merchant)
  await page.getByRole('button', { name: `Pravidelná platba ${transactionId}`, exact: true }).click()
  const dialog = page.getByRole('dialog', { name: 'Pravidelná platba' })
  await dialog.waitFor()
  await action(dialog)
  await dialog.waitFor({ state: 'hidden', timeout: 15000 })
}

async function recurringUiJourney(page) {
  await editRecurringThroughUi(page, 1010, 'Bežný návrh s kategóriou', async (dialog) => {
    await dialog.getByLabel('Interval').selectOption('yearly')
    await dialog.getByLabel('Kotva').fill('2024-01-15')
    await dialog.getByRole('button', { name: 'Uložiť', exact: true }).click()
  })
  const manual = await invoke(page, 'transaction_recurring_context', { transactionId: 1010, asOf: '2026-09-07' })
  assertEqual(manual.decision?.cadence, 'yearly', 'U01 manual recurrence through UI')

  await editRecurringThroughUi(page, 8001, 'Mesačný kotviaci obchod', async (dialog) => {
    await dialog.getByLabel('Interval').selectOption('monthly')
    await dialog.getByLabel('Kotva').fill('2026-01-31')
    await dialog.getByRole('button', { name: 'Uložiť', exact: true }).click()
  })
  const confirmed = await invoke(page, 'transaction_recurring_context', { transactionId: 8001, asOf: '2026-09-07' })
  assertEqual(confirmed.decision?.mode, 'confirmed', 'U02 confirmed state')

  await editRecurringThroughUi(page, 8001, 'Mesačný kotviaci obchod', async (dialog) => {
    await dialog.getByLabel('Interval').selectOption('quarterly')
    await dialog.getByRole('button', { name: 'Uložiť', exact: true }).click()
  })
  const edited = await invoke(page, 'transaction_recurring_context', { transactionId: 8001, asOf: '2026-09-07' })
  assertEqual(edited.decision?.cadence, 'quarterly', 'U02 edited cadence')

  await editRecurringThroughUi(page, 8001, 'Mesačný kotviaci obchod', async (dialog) => {
    await dialog.getByRole('button', { name: 'Toto nie je pravidelná platba', exact: true }).click()
  })
  const ignored = await invoke(page, 'transaction_recurring_context', { transactionId: 8001, asOf: '2026-09-07' })
  assertEqual(ignored.decision?.mode, 'ignored', 'U02 ignored state')

  await editRecurringThroughUi(page, 8001, 'Mesačný kotviaci obchod', async (dialog) => {
    await dialog.getByRole('button', { name: 'Obnoviť odhad', exact: true }).click()
  })
  const reset = await invoke(page, 'transaction_recurring_context', { transactionId: 8001, asOf: '2026-09-07' })
  assertEqual(reset.decision, null, 'U02 reset state')
  return { manual: manual.decision, confirmed: confirmed.decision, edited: edited.decision, ignored: ignored.decision, reset: reset.decision }
}

async function categoryUiJourney(page) {
  const name = 'UI syntetická kategória'
  await showTransaction(page, 1005, 'Nezaradená refundácia')
  const picker = page.getByLabel('Kategória transakcie 1005')
  await picker.selectOption('__create__')
  const createDialog = page.getByRole('dialog', { name: 'Nová kategória' })
  await createDialog.getByLabel('Názov kategórie').fill(name)
  await createDialog.getByRole('button', { name: 'Uložiť', exact: true }).click()
  await createDialog.waitFor({ state: 'hidden', timeout: 15000 })
  const categories = await invoke(page, 'list_categories')
  const created = categories.find((category) => category.name === name)
  assert(created, 'U05 new category is missing after UI creation')
  await page.waitForFunction(
    ([label, value]) => [...document.querySelectorAll('select')].some((element) => element.getAttribute('aria-label') === label && element.value === value),
    [`Kategória transakcie 1005`, String(created.id)],
  )

  await openScreen(page, 'Kategórie')
  const row = page.getByLabel(`Názov kategórie ${created.id}`).locator('xpath=..')
  await row.getByRole('button', { name: 'Upraviť', exact: true }).click()
  const editDialog = page.getByRole('dialog', { name: `Upraviť kategóriu ${name}` })
  await editDialog.getByLabel('Upraviť druh kategórie').selectOption('income')
  await editDialog.getByRole('button', { name: 'Uložiť', exact: true }).click()
  await editDialog.getByText(/Zmena druhu ovplyvní/).waitFor()
  await editDialog.getByRole('button', { name: 'Zrušiť', exact: true }).click()
  assertEqual((await invoke(page, 'list_categories')).find((category) => category.id === created.id).kind, 'expense', 'U06 cancelled kind change')
  await editDialog.getByRole('button', { name: 'Uložiť', exact: true }).click()
  await editDialog.getByRole('button', { name: 'Potvrdiť zmenu druhu', exact: true }).click()
  await editDialog.waitFor({ state: 'hidden', timeout: 15000 })
  assertEqual((await invoke(page, 'list_categories')).find((category) => category.id === created.id).kind, 'income', 'U06 confirmed kind change')

  await showTransaction(page, 8041, 'Spotify')
  const redirect = page.getByLabel('Presmerovať pravidlo transakcie 8041')
  await redirect.selectOption('401')
  await page.waitForFunction(
    () => [...document.querySelectorAll('select')].some((element) => element.getAttribute('aria-label') === 'Kategória transakcie 8041' && element.value === '401'),
  )
  const spotifyRows = await invoke(page, 'list_transactions', { filter: { text: 'Spotify' } })
  assertEqual(spotifyRows.find((row) => row.id === 8041).category_id, 401, 'U06 seed redirect through UI')
  return { created, final_kind: 'income', spotify_category_id: 401 }
}

async function expandedCommandCanaries(page) {
  const rulesBefore = await invoke(page, 'list_rules')
  const category = await invoke(page, 'save_category', { id: null, parentId: null, name: 'Samostatná syntetická kategória', kind: 'expense' })
  const rulesAfterCreate = await invoke(page, 'list_rules')
  assertEqual(rulesAfterCreate, rulesBefore, 'category creation alone must not learn a rule')

  const assignment = await invoke(page, 'assign', { ids: [1004], categoryId: category.id, applyToMatching: false })
  assert(assignment.rules_created > 0, `assigning a recognized merchant did not learn a rule: ${JSON.stringify(assignment)}`)
  const rulesAfterAssign = await invoke(page, 'list_rules')
  assert(rulesAfterAssign.length > rulesBefore.length, 'assignment did not persist learned rules')

  const categoryRequest = { id: category.id, parent_id: null, name: category.name, kind: 'income', acknowledge_kind_change: false }
  const preview = await invoke(page, 'category_update_preview', { request: categoryRequest })
  assertEqual(preview.requires_confirmation, true, 'category kind change confirmation')
  const noAck = await invokeError(page, 'update_category', { request: categoryRequest })
  const categoriesAfterNoAck = await invoke(page, 'list_categories')
  assertEqual(categoriesAfterNoAck.find((item) => item.id === category.id).kind, 'expense', 'unacknowledged kind change')
  const updated = await invoke(page, 'update_category', { request: { ...categoryRequest, acknowledge_kind_change: true } })
  assertEqual(updated.kind, 'income', 'acknowledged kind change')

  const seedRule = await invoke(page, 'seed_rule_for_transaction', { transactionId: 8041 })
  assertEqual(seedRule?.id, 402, 'Spotify seed provenance')
  const redirect = await invoke(page, 'update_rule_category', { ruleId: 402, categoryId: 401 })
  assertEqual(redirect.category_id, 401, 'Spotify seed redirect target')

  const context = await invoke(page, 'transaction_recurring_context', { transactionId: 8001, asOf: '2026-04-15' })
  assertEqual(context.inferred_cadence, 'monthly', 'transaction recurring context cadence')
  const decision = await invoke(page, 'save_recurring', {
    request: {
      decision_id: null,
      selection: { scope: 'group', transaction_id: 8001 },
      decision: { mode: 'confirmed', cadence: 'monthly', anchor_date: '2026-01-31' },
    },
  })
  const badOverlap = await invokeError(page, 'save_recurring', {
    request: {
      decision_id: null,
      selection: { scope: 'selected', transaction_ids: [8021, 8022] },
      decision: { mode: 'confirmed', cadence: 'yearly', anchor_date: '2026-01-10' },
    },
  })
  return { category, assignment, no_ack_error: noAck, updated, seedRule, redirect, context, decision, overlap_error: badOverlap }
}

async function noteAndBackupCanaries(page, oracle) {
  const note = `${'Ž'.repeat(1980)}\nFINAL-NOTE-SENTINEL`
  assertEqual([...note].length, 2000, 'native note boundary fixture')
  await invoke(page, 'save_transaction_note', { id: 1008, note })
  const tooLong = await invokeError(page, 'save_transaction_note', { id: 1008, note: `${note}X` })
  const nul = await invokeError(page, 'save_transaction_note', { id: 1008, note: 'before\0after' })
  const unknown = await invokeError(page, 'save_transaction_note', { id: 9_999_999, note: 'unknown' })
  const rows = await invoke(page, 'list_transactions', { filter: { text: 'final-note-sentinel' } })
  assertEqual(rows.map((row) => row.id), [1008], 'literal folded note search')
  const csvPath = path.join(RESULTS_DIR, 'note-literal.csv')
  const csvCount = await invoke(page, 'export_csv', { filter: { text: 'final-note-sentinel' }, path: csvPath })
  assertEqual(csvCount, 1, 'CSV literal note row count')
  const csv = fs.readFileSync(csvPath, 'utf8')
  assert(csv.includes('poznamka'), 'CSV lacks poznamka header')
  assert(csv.includes('FINAL-NOTE-SENTINEL'), 'CSV lacks exact note content')

  const backupPath = path.join(RESULTS_DIR, 'expanded-state-backup.db')
  const outcome = await invoke(page, 'backup_database', { path: backupPath })
  assertEqual(path.resolve(outcome.path), path.resolve(backupPath), 'backup path')
  const backup = databaseState(backupPath)
  assertEqual(backup.integrity, 'ok', 'backup integrity')
  assertEqual(backup.settings.schema_version, '4', 'backup schema version')
  assert(backup.tables.recurring_decisions.count >= 3, 'backup omitted recurring decisions')
  assert(backup.tables.transactions.count > 5000, 'backup omitted transactions')
  assertEqual(databaseState(oracle.db_path).integrity, 'ok', 'source integrity after backup')
  return { too_long_error: tooLong, nul_error: nul, unknown_error: unknown, searched_ids: rows.map((row) => row.id), csv: { path: csvPath, rows: csvCount, bytes: Buffer.byteLength(csv) }, backup: { path: backupPath, sha256: sha256(backupPath), state: backup } }
}

async function flowsMode() {
  const { browser, page, ready, oracle, actualDataDir } = await attach()
  try {
    fs.mkdirSync(RESULTS_DIR, { recursive: true })
    const pdfs = {}
    const dialog = await openExportDialog(page)
    for (const name of ['month', 'six_months', 'year', 'all_time']) {
      const target = path.join(RESULTS_DIR, `${name.replaceAll('_', '-')}.pdf`)
      pdfs[name] = await saveFamilyThroughUi(page, dialog, ready, name, oracle.families[name], target)
    }
    const cancel = await nativeCancel(page, dialog, ready, oracle.families.month)
    await dialog.getByRole('button', { name: 'Zrušiť', exact: true }).click()

    for (const name of ['empty', 'refunds_long_text', 'large']) {
      const target = path.join(RESULTS_DIR, `${name.replaceAll('_', '-')}.pdf`)
      pdfs[name] = await saveDirect(page, name, oracle.families[name], target)
    }

    const occupiedDialog = await openExportDialog(page)
    const occupied = await occupiedUi(page, occupiedDialog, ready, oracle.families.month, path.join(RESULTS_DIR, 'ui-occupied.pdf'))
    await occupiedDialog.getByRole('button', { name: 'Zrušiť', exact: true }).click()

    const recurringUi = await recurringUiJourney(page)
    const categoryUi = await categoryUiJourney(page)
    const fileSafety = await fileSafetyCanaries(page, oracle)
    const expanded = await expandedCommandCanaries(page)
    const notesAndBackup = await noteAndBackupCanaries(page, oracle)
    const stateAfter = databaseState(oracle.db_path)
    const result = { mode: 'flows', data_dir: actualDataDir, pdfs, cancel, occupied, recurring_ui: recurringUi, category_ui: categoryUi, file_safety: fileSafety, expanded, notes_and_backup: notesAndBackup, state_after: stateAfter }
    writeNewJson(path.join(RESULTS_DIR, 'flows.json'), result)
    writeNewJson(path.join(RESULTS_DIR, 'state-after-flows.json'), stateAfter)
    console.log(JSON.stringify({ ok: true, mode: 'flows', pdf_count: Object.keys(pdfs).length }))
  } finally {
    await browser.close()
  }
}

function normalizeExtractedText(value) {
  return value.normalize('NFC').replace(/\s+/g, ' ').trim()
}

function inspectPageManifest(name, oracle) {
  const directory = path.join(PAGE_RESULTS_DIR, name.replaceAll('_', '-'))
  const manifestPath = path.join(directory, 'manifest.json')
  assert(fs.statSync(manifestPath).isFile(), `page manifest is missing for ${name}: ${manifestPath}`)
  const manifest = readJson(manifestPath)
  const pdfPath = path.join(RESULTS_DIR, `${name.replaceAll('_', '-')}.pdf`)
  assertEqual(manifest.input_sha256, sha256(pdfPath), `${name} page manifest input hash`)
  assertEqual(manifest.page_count, manifest.pages.length, `${name} page count`)
  assertEqual(manifest.pages_with_no_text, [], `${name} pages without text`)
  assertEqual(manifest.outside_page_glyphs, 0, `${name} glyphs outside page bounds`)
  for (const [index, page] of manifest.pages.entries()) {
    assertEqual(page.page_number, index + 1, `${name} page numbering`)
    assertEqual(page.outside_page_glyphs, 0, `${name} page ${index + 1} bounds`)
    assertEqual(page.png_sha256, sha256(path.join(directory, page.png_file)), `${name} page ${index + 1} PNG hash`)
    const text = normalizeExtractedText(page.extracted_text)
    assert(text.includes(`Strana ${index + 1} z ${manifest.page_count}`), `${name} page ${index + 1} lacks exact footer`)
  }
  const text = normalizeExtractedText(manifest.pages.map((page) => page.extracted_text).join('\n'))
  const family = oracle.families[name]
  if (name === 'month') {
    for (const required of ['1 000,00 €', '100,00 €', '900,00 €', '200,00 €']) assert(text.includes(required), `month PDF lacks ${required}`)
    for (const txId of family.ordered_ids) assert(text.includes(`SYNTH-${txId}`), `month PDF lacks transaction ${txId}`)
  }
  if (name === 'all_time') {
    for (const required of Object.values(family.year_sentinels).filter((value) => typeof value === 'string')) assert(text.includes(required), `all-time PDF lacks ${required}`)
  }
  if (name === 'refunds_long_text') {
    for (const required of family.required_text) assert(text.includes(required.normalize('NFC')), `long-text PDF lacks ${required}`)
  }
  if (name === 'large') {
    const matches = text.match(/LARGE-SENTINEL-\d{5}/g) ?? []
    assertEqual(matches.length, family.sentinel.count, 'large PDF sentinel count')
    assert(text.includes(family.sentinel.first) && text.includes(family.sentinel.last), 'large PDF lacks edge sentinels')
  }
  return { manifest: manifestPath, input_sha256: manifest.input_sha256, pages: manifest.page_count, glyphs: manifest.pages.reduce((sum, page) => sum + page.glyph_count, 0) }
}

async function restartMode() {
  const { browser, page, ready, oracle, actualDataDir } = await attach()
  try {
    const expectedState = readJson(path.join(RESULTS_DIR, 'state-after-flows.json'))
    const currentState = databaseState(oracle.db_path)
    assertEqual(currentState, expectedState, 'non-audit database state after native restart')
    const previews = await previewFamilies(page, oracle)
    const recurring = await invoke(page, 'recurring_overview', { query: { from: null, to: null, account_kind: 'personal', today: oracle.today } })
    assert(recurring.rows.some((row) => row.decision === 'confirmed'), 'confirmed recurrence missing after restart')
    const pageInspection = {}
    for (const name of Object.keys(oracle.families)) pageInspection[name] = inspectPageManifest(name, oracle)
    const result = { mode: 'restart', executable: { path: ready.exe_path, sha256: ready.binary_sha256, pid: ready.pid }, data_dir: actualDataDir, state: currentState, previews, recurring_confirmed: true, page_inspection: pageInspection }
    writeNewJson(path.join(RESULTS_DIR, 'restart.json'), result)
    console.log(JSON.stringify({ ok: true, mode: 'restart', inspected_pdfs: Object.keys(pageInspection).length }))
  } finally {
    await browser.close()
  }
}

async function screenshotsMode() {
  const { browser, page, ready, actualDataDir } = await attach()
  const screenshotDir = path.join(RESULTS_DIR, 'screenshots')
  assert(!fs.existsSync(screenshotDir), `refusing existing screenshot directory: ${screenshotDir}`)
  fs.mkdirSync(screenshotDir, { recursive: true })
  const screenshots = []
  try {
    for (const theme of ['light', 'dark']) {
      await page.getByLabel('Vzhľad').selectOption(theme)
      await page.waitForFunction((value) => document.documentElement.dataset.theme === value, theme)
      for (const width of [1024, 1280]) {
        await page.setViewportSize({ width, height: 800 })
        const dialog = await openExportDialog(page)
        const target = path.join(screenshotDir, `${theme}-${width}-pdf-dialog.png`)
        await dialog.screenshot({ path: target })
        screenshots.push({ path: target, sha256: sha256(target), theme, viewport: { width, height: 800 }, state: 'PDF dialog' })
        await dialog.getByRole('button', { name: 'Zrušiť', exact: true }).click()
      }
    }
    const manifest = { mode: 'screenshots', visual_judgment: 'reserved for the routed Google reviewer', executable_sha256: ready.binary_sha256, pid: ready.pid, data_dir: actualDataDir, screenshots }
    writeNewJson(path.join(screenshotDir, 'manifest.json'), manifest)
    console.log(JSON.stringify({ ok: true, mode: 'screenshots', count: screenshots.length }))
  } finally {
    await browser.close()
  }
}

const mode = process.argv[2]
if (mode === 'inspect') await inspectMode()
else if (mode === 'flows') await flowsMode()
else if (mode === 'restart') await restartMode()
else if (mode === 'screenshots') await screenshotsMode()
else fail('usage: node acceptance/pdf-native.mjs inspect|flows|restart|screenshots')
