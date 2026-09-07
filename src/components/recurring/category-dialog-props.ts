import type { Category, CategoryKind } from '../../api'

export type RecurringCategoryDialogProps = {
  open: boolean
  initialParentId?: number | null
  initialKind?: CategoryKind
  onClose: () => void
  onCreated: (category: Category) => void
}
