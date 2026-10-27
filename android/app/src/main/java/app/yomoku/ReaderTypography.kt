package app.yomoku

import androidx.compose.ui.text.ExperimentalTextApi
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight

// One bundled variable font supplies real regular/bold outlines. Noto Serif JP has no
// italic axis; Compose synthesizes the oblique used by EPUB emphasis. API 29+ only.
@OptIn(ExperimentalTextApi::class)
internal val readerFontFamily =
    FontFamily(
        Font(
            R.font.noto_serif_jp,
            weight = FontWeight.Normal,
            variationSettings = FontVariation.Settings(FontVariation.weight(400)),
        ),
        Font(
            R.font.noto_serif_jp,
            weight = FontWeight.Bold,
            variationSettings = FontVariation.Settings(FontVariation.weight(700)),
        ),
    )
