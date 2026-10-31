package app.yomoku

internal data class AnkiProviderModelId(val value: Long) {
    init {
        require(value > 0)
    }
}

internal data class AnkiProviderModel(val id: AnkiProviderModelId, val name: String)

/**
 * Expected four-field, single forward-card layout, supplied by the future core bridge. This is
 * comparison data only: Rust owns the actual Yomoku names, HTML and CSS. Snapshot the field order
 * now so a caller's mutable list cannot change a later validation.
 */
internal class AnkiProviderModelLayout(
    val name: String,
    fieldNames: List<String>,
    val css: String,
    val cardName: String,
    val questionFormat: String,
    val answerFormat: String,
) {
    val joinedFieldNames: String

    init {
        require(name.isNotEmpty() && cardName.isNotEmpty())
        require(fieldNames.size == 4 && fieldNames.toSet().size == 4)
        require(fieldNames.all { it.isNotEmpty() && '\u001f' !in it && '\u0000' !in it })
        require(listOf(name, css, cardName, questionFormat, answerFormat).none { '\u0000' in it })
        joinedFieldNames = fieldNames.joinToString("\u001f")
    }
}
