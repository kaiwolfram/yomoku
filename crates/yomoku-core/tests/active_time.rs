use yomoku_core::active_time::{
    ActiveReadingTime, ActiveTimeError, PendingReadingTime, IDLE_TIMEOUT_MS,
};

#[derive(Default)]
struct FakeClock(u64);

impl FakeClock {
    fn now(&self) -> u64 {
        self.0
    }

    fn advance(&mut self, milliseconds: u64) -> u64 {
        self.0 += milliseconds;
        self.now()
    }
}

fn pending(milliseconds: i64) -> PendingReadingTime {
    PendingReadingTime::from_millis(milliseconds).unwrap()
}

#[test]
fn reading_starts_on_open_and_stops_exactly_at_five_idle_minutes() {
    let mut clock = FakeClock(42_000);
    let mut timer = ActiveReadingTime::default();
    assert_eq!(timer.resume(clock.now()), Ok(pending(0)));
    assert_eq!(
        timer.checkpoint(clock.advance(299_999)),
        Ok(pending(299_999))
    );
    assert_eq!(timer.checkpoint(clock.advance(1)), Ok(pending(300_000)));
    assert_eq!(timer.checkpoint(clock.advance(1)), Ok(pending(300_000)));
    assert_eq!(
        timer.checkpoint(clock.advance(3_600_000)),
        Ok(pending(300_000))
    );
    assert_eq!(timer.pause(clock.now()), Ok(pending(300_000)));
}

#[test]
fn interactions_extend_reading_and_restart_after_idle_without_counting_the_gap() {
    let mut clock = FakeClock::default();
    let mut timer = ActiveReadingTime::default();
    timer.resume(clock.now()).unwrap();
    assert_eq!(timer.interact(clock.advance(240_000)), Ok(pending(240_000)));
    assert_eq!(timer.interact(clock.advance(300_000)), Ok(pending(540_000)));
    // The first five minutes since the last interaction count, the rest do not.
    assert_eq!(timer.interact(clock.advance(900_000)), Ok(pending(840_000)));
    assert_eq!(timer.pause(clock.advance(20_000)), Ok(pending(860_000)));
}

#[test]
fn dictionary_popups_and_their_interactions_remain_active_reading() {
    let mut clock = FakeClock::default();
    let mut timer = ActiveReadingTime::default();
    timer.resume(clock.now()).unwrap();
    timer.interact(clock.advance(20_000)).unwrap(); // Open dictionary.
    assert_eq!(
        timer.checkpoint(clock.advance(180_000)),
        Ok(pending(200_000))
    );
    timer.interact(clock.now()).unwrap(); // Interact within dictionary.
    timer.interact(clock.advance(180_000)).unwrap(); // Dismiss dictionary.
    assert_eq!(timer.pause(clock.advance(20_000)), Ok(pending(400_000)));
}

#[test]
fn background_and_reader_close_pause_until_explicit_resume() {
    let mut clock = FakeClock::default();
    let mut timer = ActiveReadingTime::default();
    timer.resume(clock.now()).unwrap();
    assert_eq!(timer.pause(clock.advance(12_000)), Ok(pending(12_000))); // Background.
    assert_eq!(
        timer.checkpoint(clock.advance(3_600_000)),
        Ok(pending(12_000))
    );
    timer.resume(clock.now()).unwrap();
    assert_eq!(timer.pause(clock.advance(18_000)), Ok(pending(30_000))); // Close.
    timer.pause(clock.advance(3_600_000)).unwrap();
    timer.resume(clock.now()).unwrap();
    assert_eq!(timer.checkpoint(clock.advance(10_000)), Ok(pending(40_000)));
}

#[test]
fn duplicate_lifecycle_notifications_do_not_extend_idle_or_double_count() {
    let mut timer = ActiveReadingTime::default();
    timer.resume(0).unwrap();
    assert_eq!(timer.resume(200_000), Ok(pending(200_000)));
    assert_eq!(timer.resume(400_000), Ok(pending(300_000)));
    assert_eq!(timer.pause(500_000), Ok(pending(300_000)));
    assert_eq!(timer.pause(600_000), Ok(pending(300_000)));
    timer.resume(600_000).unwrap();
    timer.resume(600_000).unwrap();
    assert_eq!(timer.pause(610_000), Ok(pending(310_000)));
}

#[test]
fn checkpoints_neither_reset_idle_nor_pause_or_drain_pending_time() {
    let mut timer = ActiveReadingTime::default();
    timer.resume(0).unwrap();
    let saved = timer.checkpoint(50_000).unwrap();
    assert_eq!(saved, pending(50_000));
    assert_eq!(timer.checkpoint(150_000), Ok(pending(150_000)));
    assert_eq!(timer.checkpoint(350_000), Ok(pending(300_000)));
    assert_eq!(timer.checkpoint(350_000), Ok(pending(300_000)));
    // A saved/frozen value is stable while subsequent accounting continues.
    assert_eq!(saved, pending(50_000));
}

#[test]
fn saved_partial_time_restores_paused_across_close_process_restart_and_reboot() {
    for new_epoch in [0, 9_000_000] {
        let mut old = ActiveReadingTime::default();
        old.resume(1_000_000).unwrap();
        old.interact(1_240_000).unwrap();
        let saved = old.pause(1_300_000).unwrap();
        assert_eq!(saved, pending(300_000));

        // Persist only milliseconds, not clock readings or an active flag.
        let restored = pending(saved.milliseconds());
        let mut timer = ActiveReadingTime::from_pending(restored);
        assert_eq!(timer.checkpoint(new_epoch), Ok(saved));
        timer.resume(new_epoch + 60_000).unwrap();
        assert_eq!(timer.pause(new_epoch + 80_000), Ok(pending(320_000)));
    }
}

#[test]
fn restoring_a_checkpoint_does_not_count_time_since_the_last_save() {
    let mut timer = ActiveReadingTime::default();
    timer.resume(1_000).unwrap();
    let saved = timer.checkpoint(11_000).unwrap();
    // Simulate process death without pause: only the last durable value survives.
    let mut restored = ActiveReadingTime::from_pending(saved);
    assert_eq!(restored.checkpoint(9_000_000), Ok(pending(10_000)));
    restored.resume(9_000_000).unwrap();
    assert_eq!(restored.pause(9_005_000), Ok(pending(15_000)));
}

#[test]
fn interactions_outside_the_foreground_reader_never_start_the_timer() {
    let mut timer = ActiveReadingTime::from_pending(pending(123));
    timer.interact(0).unwrap();
    assert_eq!(timer.checkpoint(60_000), Ok(pending(123)));
    timer.resume(60_000).unwrap();
    timer.pause(61_000).unwrap();
    timer.interact(62_000).unwrap();
    assert_eq!(timer.checkpoint(120_000), Ok(pending(1_123)));
}

#[test]
fn clock_regression_is_rejected_for_every_action_without_mutation() {
    let mut timer = ActiveReadingTime::default();
    timer.resume(1_000).unwrap();
    timer.checkpoint(2_000).unwrap();
    let before = timer.clone();
    assert_eq!(
        timer.resume(1_999),
        Err(ActiveTimeError::ClockWentBackwards)
    );
    assert_eq!(timer, before);
    assert_eq!(timer.pause(1_999), Err(ActiveTimeError::ClockWentBackwards));
    assert_eq!(timer, before);
    assert_eq!(
        timer.interact(1_999),
        Err(ActiveTimeError::ClockWentBackwards)
    );
    assert_eq!(timer, before);
    assert_eq!(
        timer.checkpoint(1_999),
        Err(ActiveTimeError::ClockWentBackwards)
    );
    assert_eq!(timer, before);
    assert_eq!(timer.pause(3_000), Ok(pending(2_000)));
    let before = timer.clone();
    assert_eq!(timer.resume(0), Err(ActiveTimeError::ClockWentBackwards));
    assert_eq!(timer, before);
}

#[test]
fn invalid_or_overflowing_durations_are_rejected_without_losing_pending_time() {
    assert_eq!(
        PendingReadingTime::from_millis(-1),
        Err(ActiveTimeError::NegativeDuration)
    );
    assert_eq!(
        PendingReadingTime::from_millis(i64::MIN),
        Err(ActiveTimeError::NegativeDuration)
    );
    let mut timer = ActiveReadingTime::from_pending(pending(i64::MAX - 10));
    timer.resume(0).unwrap();
    let before = timer.clone();
    assert_eq!(timer.pause(11), Err(ActiveTimeError::DurationOverflow));
    assert_eq!(timer, before);
    assert_eq!(timer.pause(10), Ok(pending(i64::MAX)));
    assert_eq!(timer.checkpoint(u64::MAX), Ok(pending(i64::MAX)));
}

#[test]
fn idle_deadline_near_maximum_clock_value_does_not_overflow() {
    let mut timer = ActiveReadingTime::default();
    timer.resume(u64::MAX - IDLE_TIMEOUT_MS - 10).unwrap();
    assert_eq!(timer.checkpoint(u64::MAX - 5), Ok(pending(300_000)));
    timer.interact(u64::MAX - 5).unwrap();
    assert_eq!(timer.pause(u64::MAX), Ok(pending(300_005)));
    let mut timer = ActiveReadingTime::default();
    timer.resume(0).unwrap();
    assert_eq!(timer.pause(u64::MAX), Ok(pending(300_000)));
}

#[test]
fn accounting_is_independent_of_checkpoint_frequency() {
    // Identical interaction/lifecycle timelines, with optional intermediate
    // checkpoints at arbitrary intervals, must produce identical totals.
    for interval in [1, 17, 999, 60_000, 300_000, 900_000] {
        let mut timer = ActiveReadingTime::default();
        timer.resume(0).unwrap();
        let mut now = interval;
        while now < 400_000 {
            timer.checkpoint(now).unwrap();
            now += interval;
        }
        timer.interact(400_000).unwrap();
        now = 400_000 + interval;
        while now < 650_000 {
            timer.checkpoint(now).unwrap();
            now += interval;
        }
        assert_eq!(timer.pause(650_000), Ok(pending(550_000)));
    }
}
