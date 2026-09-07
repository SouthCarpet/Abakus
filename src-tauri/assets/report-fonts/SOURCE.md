# Bundled report fonts

Abakus bundles the unchanged hinted TrueType files `NotoSans-Regular.ttf` and
`NotoSans-Bold.ttf` from the archived upstream `notofonts/noto-fonts`
repository. Both files identify themselves as Noto Sans version 2.008. They
were retrieved from upstream commit
`ffebf8c1ee449e544955a7e813c54f9b73848eac` on 2026-09-07.

Exact sources:

- `https://raw.githubusercontent.com/notofonts/noto-fonts/ffebf8c1ee449e544955a7e813c54f9b73848eac/hinted/ttf/NotoSans/NotoSans-Regular.ttf`
- `https://raw.githubusercontent.com/notofonts/noto-fonts/ffebf8c1ee449e544955a7e813c54f9b73848eac/hinted/ttf/NotoSans/NotoSans-Bold.ttf`
- `https://raw.githubusercontent.com/notofonts/noto-fonts/ffebf8c1ee449e544955a7e813c54f9b73848eac/LICENSE`

SHA-256:

- `NotoSans-Regular.ttf`: `B85C38ECEA8A7CFB39C24E395A4007474FA5A4FC864F6EE33309EB4948D232D5`
- `NotoSans-Bold.ttf`: `C976E4B1B99EDC88775377FCC21692CA4BFA46B6D6CA6522BFDA505B28FF9D6A`
- `OFL.txt`: `0DAB92D0544F7B233403F14B84A663BDBFA746982EDA629E7F4F9FFE1B036FEB`

The included `OFL.txt` is the upstream SIL Open Font License 1.1. It permits
embedding and redistribution with the license text. The app embeds both font
files at compile time. It does not download or look up fonts at runtime.
