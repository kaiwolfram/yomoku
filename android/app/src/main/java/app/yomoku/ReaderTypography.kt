package app.yomoku

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.text.ExperimentalTextApi
import androidx.compose.ui.text.font.Font
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontVariation
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp
import app.yomoku.shared.ReaderFontSize
import app.yomoku.shared.ReaderLineSpacing
import app.yomoku.shared.ReaderTypography

internal val ReaderFontSize.spValue: Int
    get() =
        when (this) {
            ReaderFontSize.SP14 -> 14
            ReaderFontSize.SP16 -> 16
            ReaderFontSize.SP18 -> 18
            ReaderFontSize.SP20 -> 20
            ReaderFontSize.SP24 -> 24
        }

internal val ReaderLineSpacing.percent: Int
    get() =
        when (this) {
            ReaderLineSpacing.PERCENT125 -> 125
            ReaderLineSpacing.PERCENT150 -> 150
            ReaderLineSpacing.PERCENT175 -> 175
            ReaderLineSpacing.PERCENT200 -> 200
        }

@Composable
internal fun readerTextStyle(typography: ReaderTypography?) =
    MaterialTheme.typography.bodyLarge.copy(
        fontFamily = readerFontFamily,
        fontSize = (typography?.fontSize?.spValue ?: 16).sp,
        lineHeight =
            ((typography?.fontSize?.spValue ?: 16) * (typography?.lineSpacing?.percent ?: 150) /
                    100f)
                .sp,
    )

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
