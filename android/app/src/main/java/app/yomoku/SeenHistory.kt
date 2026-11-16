package app.yomoku

/** Saved reading dates only; callers may inject time for deterministic rendering/tests. */
internal fun seenHistory(
    count: ULong,
    lastSeenUnixMs: Long?,
    nowUnixMs: Long = System.currentTimeMillis(),
): String {
    val seen = "Seen $count ${if (count == 1UL) "time" else "times"}"
    if (lastSeenUnixMs == null) return seen
    val elapsed = if (nowUnixMs <= lastSeenUnixMs) 0L else nowUnixMs - lastSeenUnixMs
    val units =
        listOf(
            365L * 24 * 60 * 60 * 1000 to "year",
            30L * 24 * 60 * 60 * 1000 to "month",
            7L * 24 * 60 * 60 * 1000 to "week",
            24L * 60 * 60 * 1000 to "day",
            60L * 60 * 1000 to "hour",
            60L * 1000 to "minute",
        )
    val unit = units.firstOrNull { elapsed >= it.first } ?: return "$seen · just now"
    val amount = elapsed / unit.first
    return "$seen · $amount ${unit.second}${if (amount == 1L) "" else "s"} ago"
}
