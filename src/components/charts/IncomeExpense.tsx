import { Bar, BarChart, CartesianGrid, Legend, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts'
import type { Summary } from '../../api'
import { formatEur } from '../../api'

export function IncomeExpense({ rows }: { rows: Summary['by_month'] }) {
  return (
    <div className="k-well" style={{ padding: 'var(--space-3)' }}>
      <ResponsiveContainer width="100%" height={220}>
        <BarChart data={rows}>
          <CartesianGrid stroke="var(--color-chart-grid)" vertical={false} />
          <XAxis dataKey="month" stroke="var(--color-text-muted)" fontSize={12} tickLine={false} />
          <YAxis stroke="var(--color-text-muted)" fontSize={12} tickLine={false} domain={[0, 'auto']} tickFormatter={(v: number) => formatEur(v)} />
          <Tooltip formatter={(value) => formatEur(Number(value))} />
          <Legend />
          <Bar dataKey="income_cents" name="Príjem" fill="var(--color-chart-1)" />
          <Bar dataKey="expense_cents" name="Výdavky" fill="var(--color-chart-2)" />
        </BarChart>
      </ResponsiveContainer>
    </div>
  )
}
