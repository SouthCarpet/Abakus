# PDF export UI, 0.1.2

This lane provides the native PDF export action and dialog. It does not make a
browser PDF or use a transaction table filter.

The controller mounts it beside the theme picker:

```tsx
import { ExportPdfAction } from './components/report/ExportPdfAction'

<div className="k-app-toolbar"><ThemePicker /><ExportPdfAction /></div>
```

`report.css` supplies `.k-app-toolbar` only if the controller needs the two
toolbar controls grouped. The export dialog styles are loaded by
`ExportPdfAction` itself.

The UI calls only these native command names:

- `preview_pdf_report` with `{ request }`
- `export_pdf_report` with `{ request, path }`

The dialog starts with the current local calendar month and all accounts. It
offers month, six consecutive months ending in the selected month, year, and
all time. A preview is required before export. It shows captured bounds,
scope, selected accounts, transaction count, latest transaction, capture time,
unfinished-period notice, and coverage warnings.

During native save selection and report generation, the selectors and close
actions remain disabled. A cancelled save creates no result. An error keeps
the draft in place. A successful message uses the native outcome capture, not
the preview count.

Focused UI tests are in `ExportPdfDialog.test.tsx` and map to P02, P03 and
P16. They use literal fake accounts and native-boundary doubles. The full
desktop and rendered-PDF acceptance remains the controller and independent
verifier task after the Rust renderer is integrated.
