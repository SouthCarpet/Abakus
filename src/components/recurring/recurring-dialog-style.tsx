export function RecurringWideDialogStyle() {
  return (
    <style>{'.k-dialog:has([data-recurring-wide]){width:min(90vw, calc(var(--space-8) * 30));max-height:80vh;overflow:auto;}'}</style>
  )
}
