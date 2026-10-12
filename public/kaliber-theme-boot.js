/* First paint (Kaliber S2, spec R20): set the saved theme before the body exists, so a saved
   dark theme does not flash light. Same rule as src/kaliber/theme.ts: the shared key
   kaliber-theme, else the old Abakus key abakus.theme (read once; theme.ts migrates it), else
   the system setting. A file and not an inline script: the CSP is script-src 'self'.
   It runs no timer and moves nothing. */
(function () {
  try {
    var store = window.localStorage;
    var saved = store.getItem('kaliber-theme') || store.getItem('abakus.theme');
    var dark = saved === 'dark' || (saved !== 'light' && window.matchMedia('(prefers-color-scheme: dark)').matches);
    document.documentElement.dataset.theme = dark ? 'dark' : 'light';
  } catch (e) {
    /* storage blocked: the app sets the theme after mount */
  }
})();
