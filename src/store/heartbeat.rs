use std::pin::Pin;
use std::time::Duration;
use tokio::time::{sleep, Instant, Sleep};

/// Generates a random timeout duration within [timeout, 2 * timmeout].
fn randomize_timeout_duration(timeout: u64) -> Duration {
    Duration::from_millis(rand::random_range(timeout..=(2 * timeout)))
}

/// Creates a heartbeat future with a random deadline within the timeout range.
pub fn new_random(timeout: u64) -> Sleep {
    sleep(randomize_timeout_duration(timeout))
}

/// Resets an existing heartbeat future.
pub fn reset(heartbeat: Pin<&mut Sleep>, timeout: u64) {
    let timeout_duration = randomize_timeout_duration(timeout);
    let deadline = Instant::now()
        .checked_add(timeout_duration)
        .expect("should be able to create new deadline");
    heartbeat.reset(deadline);
}
