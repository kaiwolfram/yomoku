package app.yomoku

internal fun verifySeenHistory() {
    val minute = 60_000L
    val hour = 60 * minute
    val day = 24 * hour
    val now = 1_800_000_000_000L
    check(seenHistory(0UL, null, now) == "Seen 0 times")
    check(seenHistory(1UL, null, now) == "Seen 1 time")
    check(seenHistory(ULong.MAX_VALUE, null, now) == "Seen ${ULong.MAX_VALUE} times")
    val thresholds =
        listOf(
            0L to "just now",
            minute - 1 to "just now",
            minute to "1 minute ago",
            2 * minute to "2 minutes ago",
            hour - 1 to "59 minutes ago",
            hour to "1 hour ago",
            day - 1 to "23 hours ago",
            day to "1 day ago",
            7 * day - 1 to "6 days ago",
            7 * day to "1 week ago",
            21 * day to "3 weeks ago",
            30 * day - 1 to "4 weeks ago",
            30 * day to "1 month ago",
            365 * day - 1 to "12 months ago",
            365 * day to "1 year ago",
            2 * 365 * day to "2 years ago",
        )
    for ((elapsed, label) in thresholds) {
        check(seenHistory(5UL, now - elapsed, now) == "Seen 5 times · $label")
    }
    check(seenHistory(1UL, now + day, now) == "Seen 1 time · just now")
}
