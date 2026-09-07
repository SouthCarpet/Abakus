import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { recurringApi } from '../../lib/recurring-api'
import { RecurringPanel } from './RecurringPanel'
import { deferred, EMPTY_OVERVIEW, R14_ESTIMATE, R14_OVERVIEW, recurringRow, txRow } from './oracles'

afterEach(() => cleanup())

vi.mock('../../lib/recurring-api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../lib/recurring-api')>()
  return {
    ...actual,
    recurringApi: {
      overview: vi.fn(),
      detail: vi.fn(),
      transactionContext: vi.fn(),
      save: vi.fn(),
      reset: vi.fn(),
    },
  }
})

vi.mock('../../api', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../api')>()
  return { ...actual, api: { listCategories: vi.fn().mockResolvedValue([]), assign: vi.fn() } }
})

beforeEach(() => {
  vi.mocked(recurringApi.overview).mockReset().mockResolvedValue(EMPTY_OVERVIEW)
  vi.mocked(recurringApi.detail).mockReset()
  vi.mocked(recurringApi.save).mockReset()
  vi.mocked(recurringApi.reset).mockReset()
})

const ALL = { kind: 'all' as const }

describe('RecurringPanel loading and failure never look like a zero total', () => {
  it('shows a pending status and no 0,00 € KPIs while overview is in flight', () => {
    vi.mocked(recurringApi.overview).mockReturnValue(new Promise(() => {}))
    render(<RecurringPanel period={ALL} accountKind="all" today="2026-09-07" />)
    expect(screen.getByRole('status')).toHaveTextContent('Načítavajú sa pravidelné platby')
    expect(screen.queryByText('0,00 €')).not.toBeInTheDocument()
    expect(screen.queryByText('Pravidelné výdavky za mesiac')).not.toBeInTheDocument()
  })

  it('shows the backend error and no zero KPI totals when the query fails', async () => {
    vi.mocked(recurringApi.overview).mockRejectedValue(new Error('offline'))
    render(<RecurringPanel period={ALL} accountKind="all" today="2026-09-07" />)
    expect(await screen.findByRole('alert')).toHaveTextContent('offline')
    expect(screen.queryByText('0,00 €')).not.toBeInTheDocument()
    expect(screen.queryByText('Pravidelné výdavky za mesiac')).not.toBeInTheDocument()
  })
})

describe('RecurringPanel empty and future states', () => {
  it('explains the observation minimum and that manual setup starts in a transaction detail', async () => {
    render(<RecurringPanel period={ALL} accountKind="all" today="2026-09-07" />)
    expect(await screen.findByText(/tri mesačné/)).toBeInTheDocument()
    expect(screen.getByText(/detaile transakcie/)).toBeInTheDocument()
  })

  it('does not present current totals under a future-period heading', async () => {
    vi.mocked(recurringApi.overview).mockResolvedValue({
      ...EMPTY_OVERVIEW,
      future_period: true,
      unfinished_period: false,
      as_of: '2026-10-31',
    })
    render(<RecurringPanel period={{ kind: 'custom', custom: { from: '2026-10-01', to: '2026-10-31' } }} accountKind="all" today="2026-09-07" />)
    expect(await screen.findByText(/v budúcnosti/)).toBeInTheDocument()
    expect(screen.queryByText('Pravidelné výdavky za mesiac')).not.toBeInTheDocument()
  })

  it('marks an unfinished current period', async () => {
    vi.mocked(recurringApi.overview).mockResolvedValue(R14_OVERVIEW)
    render(<RecurringPanel period={ALL} accountKind="all" today="2026-09-01" />)
    expect(await screen.findByText('Obdobie ešte neskončilo')).toBeInTheDocument()
  })
})

describe('RecurringPanel R14/R15 rendering and honest labels', () => {
  it('shows confirmed versus estimate totals and remaining-month charges from the R14 oracle', async () => {
    vi.mocked(recurringApi.overview).mockResolvedValue(R14_OVERVIEW)
    render(<RecurringPanel period={ALL} accountKind="all" today="2026-09-01" />)
    expect(await screen.findByText('53,67 €')).toBeInTheDocument()
    expect(screen.getByText('2 446,33 €')).toBeInTheDocument()
    expect(screen.getByText('Ďalšie odhady 5,00 €')).toBeInTheDocument()
    expect(screen.getByText('Potvrdené výdavky 12,00 €')).toBeInTheDocument()
    expect(screen.getByText('Hosting')).toBeInTheDocument()
    expect(screen.getByText('4,88 % priemerných mesačných výdavkov 1 100,00 € (2026-07, 2026-08).')).toBeInTheDocument()
  })

  it('leaves remaining 12,00 € in place after switching to the annual projection', async () => {
    vi.mocked(recurringApi.overview).mockResolvedValue(R14_OVERVIEW)
    render(<RecurringPanel period={ALL} accountKind="all" today="2026-09-01" />)
    await screen.findByText('53,67 €')
    fireEvent.click(screen.getByRole('button', { name: 'Za rok' }))
    expect(screen.getByText('644,04 €')).toBeInTheDocument()
    expect(screen.getByText('Potvrdené výdavky 12,00 €')).toBeInTheDocument()
    expect(screen.getByText('Potvrdené príjmy 2 500,00 €')).toBeInTheDocument()
  })

  it('shows a yearly apportionment label from backend monthly cents, not a float split', async () => {
    vi.mocked(recurringApi.overview).mockResolvedValue(R14_OVERVIEW)
    render(<RecurringPanel period={ALL} accountKind="all" today="2026-09-01" />)
    expect(await screen.findByText('8,33 € mesačne (ročne)')).toBeInTheDocument()
    expect(screen.getByText('100,00 €')).toBeInTheDocument()
  })

  it('keeps ignored rows out of totals and under a collapsed Ignorované section', async () => {
    vi.mocked(recurringApi.overview).mockResolvedValue({
      ...R14_OVERVIEW,
      rows: [...R14_OVERVIEW.rows, recurringRow({ series_key: 'g:ign', decision: 'ignored', name: 'Obchod', decision_id: 9 })],
    })
    render(<RecurringPanel period={ALL} accountKind="all" today="2026-09-01" />)
    await screen.findByText('53,67 €')
    expect(screen.getByText('Ignorované (1)')).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Detail Obchod' })).not.toBeVisible()
    fireEvent.click(screen.getByText('Ignorované (1)'))
    expect(screen.getByRole('button', { name: 'Detail Obchod' })).toBeVisible()
  })

  it('labels a price change and a foreign EUR estimate in text, not by color alone', async () => {
    vi.mocked(recurringApi.overview).mockResolvedValue({
      ...EMPTY_OVERVIEW,
      unfinished_period: false,
      rows: [
        recurringRow({
          name: 'Stream',
          price_change: { currency: 'USD', previous_cents: 1000, current_cents: 1200, delta_cents: 200, effective_from: '2026-05-01' },
          currency: 'USD',
          original_amount_cents: 1200,
          amount_cents: 1050,
          foreign_eur_estimate: true,
        }),
      ],
    })
    render(<RecurringPanel period={ALL} accountKind="all" today="2026-09-07" />)
    expect(await screen.findByText('Zdraželo o 2,00 USD od 2026-05')).toBeInTheDocument()
    expect(screen.getByText('Prepočet podľa poslednej platby; kurz sa môže zmeniť')).toBeInTheDocument()
    expect(screen.getByText('12,00 USD (10,50 €)')).toBeInTheDocument()
  })

  it('names a historical remaining month instead of this month', async () => {
    vi.mocked(recurringApi.overview).mockResolvedValue({ ...R14_OVERVIEW, as_of: '2026-08-31', unfinished_period: false })
    render(<RecurringPanel period={ALL} accountKind="all" today="2026-09-07" />)
    expect(await screen.findByText('Zostávalo v auguste 2026')).toBeInTheDocument()
    expect(screen.queryByText('Ešte príde tento mesiac')).not.toBeInTheDocument()
  })

  it('labels unknown/missing_coverage instead of calling it a missing payment', async () => {
    vi.mocked(recurringApi.overview).mockResolvedValue({
      ...EMPTY_OVERVIEW,
      unfinished_period: false,
      excluded: { missing: 0, ended: 0, unknown: 1 },
      rows: [recurringRow({ name: 'Nájom', state: 'unknown', unknown_reason: 'missing_coverage' })],
    })
    render(<RecurringPanel period={ALL} accountKind="all" today="2026-09-07" />)
    expect(await screen.findByText('Neznáme, chýba výpis')).toBeInTheDocument()
    expect(screen.queryByText('Chýba platba')).not.toBeInTheDocument()
    expect(screen.getByText('Mimo súčtov: chýbajúce 0, odhad ukončenia 0, neznáme 1.')).toBeInTheDocument()
  })
})

describe('RecurringPanel generation guard U04', () => {
  it('does not let a slower earlier overview overwrite a later account-kind selection', async () => {
    const first = deferred<typeof R14_OVERVIEW>()
    const second = deferred<typeof R14_OVERVIEW>()
    vi.mocked(recurringApi.overview).mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise)
    const { rerender } = render(<RecurringPanel period={ALL} accountKind="all" today="2026-09-07" />)
    rerender(<RecurringPanel period={ALL} accountKind="personal" today="2026-09-07" />)
    second.resolve({ ...EMPTY_OVERVIEW, rows: [recurringRow({ name: 'FRESH' })] })
    expect(await screen.findByText('FRESH')).toBeInTheDocument()
    first.resolve({ ...EMPTY_OVERVIEW, rows: [recurringRow({ name: 'STALE' })] })
    await waitFor(() => expect(screen.getByText('FRESH')).toBeInTheDocument())
    expect(screen.queryByText('STALE')).not.toBeInTheDocument()
  })
})

describe('RecurringPanel confirm, ignore, reset and saved-refresh failure', () => {
  it('confirms an estimate with the row cadence and does not invent a second save on refresh retry', async () => {
    vi.mocked(recurringApi.overview).mockResolvedValue({
      ...EMPTY_OVERVIEW,
      unfinished_period: false,
      rows: [R14_ESTIMATE],
    })
    vi.mocked(recurringApi.detail).mockResolvedValue({
      row: R14_ESTIMATE,
      transactions: [txRow({ id: 44 })],
      matching_transaction_ids: [44],
      compatible_transactions: [],
    })
    vi.mocked(recurringApi.save).mockResolvedValue({
      id: 8, series_key: R14_ESTIMATE.series_key, group_key: R14_ESTIMATE.group_key, account_id: 1,
      scope: 'group', mode: 'confirmed', cadence: 'monthly', anchor_date: '2026-01-15', updated_at: '2026-09-01T00:00:00Z',
    })
    vi.mocked(recurringApi.overview)
      .mockResolvedValueOnce({ ...EMPTY_OVERVIEW, unfinished_period: false, rows: [R14_ESTIMATE] })
      .mockRejectedValueOnce(new Error('obnova zlyhala'))
      .mockResolvedValueOnce({ ...EMPTY_OVERVIEW, unfinished_period: false, rows: [recurringRow({ ...R14_ESTIMATE, decision: 'confirmed', decision_id: 8, name: 'Hosting' })] })

    render(<RecurringPanel period={ALL} accountKind="all" today="2026-09-01" />)
    fireEvent.click(await screen.findByRole('button', { name: 'Potvrdiť Hosting' }))
    expect(await screen.findByRole('alert')).toHaveTextContent('Uložené, obnovenie zlyhalo')
    expect(recurringApi.save).toHaveBeenCalledTimes(1)
    expect(recurringApi.save).toHaveBeenCalledWith({
      decision_id: null,
      selection: { scope: 'group', transaction_id: 44 },
      decision: { mode: 'confirmed', cadence: 'monthly', anchor_date: R14_ESTIMATE.anchor_date },
    })
    fireEvent.click(screen.getByRole('button', { name: 'Obnoviť prehľad' }))
    await waitFor(() => expect(screen.queryByText('Uložené, obnovenie zlyhalo')).not.toBeInTheDocument())
    expect(recurringApi.save).toHaveBeenCalledTimes(1)
  })
})
