import type { Account, AccountKind, StatementHistoryRow } from '../../api'
import { buildStatementReminders, type StatementReminder } from '../../lib/insights/statement-reminders'
import { formatDate } from '../../lib/format'
import { Button } from '../Button'
import { Card } from '../Card'

// A gap here is a hole in statement coverage, not proof that transactions are
// missing: the bank may simply not have issued or provided that statement
// yet. Every line says so, so the wording never reads as an accusation.
function reminderLine(accountLabel: string, from: string, to: string): string {
  return `${accountLabel}: chýba výpis za obdobie ${formatDate(from)}–${formatDate(to)}. Toto je medzera v pokrytí výpismi, nie dôkaz chýbajúcich transakcií.`
}

function reminderLines(reminder: StatementReminder): { key: string; text: string }[] {
  return reminder.missingRanges.map((range) => ({
    key: `${reminder.accountId}-${range.from}`,
    text: reminderLine(reminder.accountLabel, range.from, range.to),
  }))
}

export function StatementReminderBanner({
  accounts,
  statements,
  accountKind,
  today,
  onNavigateToImport,
}: {
  accounts: readonly Pick<Account, 'id' | 'label' | 'kind'>[]
  statements: readonly StatementHistoryRow[]
  accountKind: AccountKind | null
  today: string
  onNavigateToImport: () => void
}) {
  const { reminders } = buildStatementReminders(accounts, statements, accountKind, today)
  const lines = reminders.flatMap(reminderLines)
  if (lines.length === 0) return null

  return (
    <Card
      title="Pripomienka pokrytia výpismi"
      footer={
        <Button variant="secondary" onClick={onNavigateToImport}>
          Prejsť na import
        </Button>
      }
    >
      {lines.map((line) => (
        <p key={line.key}>{line.text}</p>
      ))}
    </Card>
  )
}
