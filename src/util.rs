/// Sleep until the next clock-aligned tick.
///
/// For example, with `interval_secs = 300` (5 min), if the current time is
/// 14:03:22, this sleeps until 14:05:00. With `interval_secs = 1800` (30 min),
/// it sleeps until 14:30:00.
pub async fn sleep_until_aligned(interval_secs: u64) {
    let now = chrono::Utc::now();
    let current_secs = now.timestamp() as u64;
    let next_tick = (current_secs / interval_secs + 1) * interval_secs;
    let sleep_secs = next_tick - current_secs;

    tracing::debug!(
        next_tick_in_secs = sleep_secs,
        next_tick_at = %chrono::DateTime::from_timestamp(next_tick as i64, 0).unwrap_or_default(),
        "sleeping until next aligned tick"
    );

    tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;
}
