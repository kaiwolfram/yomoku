# Noto Serif JP

The reader bundles the unmodified Noto Serif JP variable TrueType font, version
2.003, from [Google Fonts](https://github.com/google/fonts/tree/8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5/ofl/notoserifjp)
at revision `8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5`. The pinned
[METADATA.pb](METADATA.pb) identifies the upstream Noto CJK source revision and
200–900 weight axis. No subsetting/conversion or runtime download is performed.

The [font](../../android/app/src/main/res/font/noto_serif_jp.ttf) contains
13,574,352 bytes. Its internal copyright is `(c) 2017-2024 Adobe
(http://www.adobe.com/).`; the upstream OFL notice also credits Google. Complete
[attribution](../../android/app/src/main/assets/noto-serif-jp/ATTRIBUTION.txt) and
[SIL OFL 1.1](../../android/app/src/main/assets/noto-serif-jp/OFL.txt) are bundled
and available offline from Settings → Reader font credits.

SHA-256:

- Font: `2fd527ba12b6a44ec30d796d633360da0aeba6c5d4af1304ce12bb4dc15a7dfc`
- OFL.txt: `5e0da210fb04058a8c0087985d2d456b931c2579811a49655721d3cf0c36b6d6`
- METADATA.pb: `265cb1cd38f7c12837feb5b26c59328ef7ff8f0b2b5683b1a47303c68d373622`

To reproduce the download, use the pinned directory on
`https://raw.githubusercontent.com/google/fonts/8b0a1d0f5983c89bc2b93f1b5fb55f9e252744b5/ofl/notoserifjp/`
and files `NotoSerifJP%5Bwght%5D.ttf`, `OFL.txt` and `METADATA.pb`.

Compose regular/bold entries explicitly set weight-axis values 400/700 in the same
resource; EPUB emphasis uses synthesized italic because this font has no italic
axis. The existing API-29 minimum covers variable font support (API 26+). The
resource is used by plain/tokenized reader text and fragments around images.
Android supplies glyph fallback for characters absent from the font (e.g. emoji).
Logical pages/UTF-8 anchors remain core-owned and independent of font metrics.

API verified against [Android's Compose fonts documentation](https://developer.android.com/develop/ui/compose/text/fonts),
using the existing Compose BOM 2026.03.01/ui-text 1.10.6; no new dependency.
