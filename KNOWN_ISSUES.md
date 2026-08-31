# Known issues

- **pdfium binary: trust on first use.** `scripts/fetch-pdfium.ps1` pins a release tag
  (`chromium/7469`) from `bblanchon/pdfium-binaries` and checks the archive against
  `scripts/pdfium.sha256`. The hash file was written on the first run, from that same
  download. This proves the binary stays the same across re-runs. It does not prove the
  binary was safe on day one. Treat the pinned hash as trust-on-first-use, not as an
  independent security check.
