//! Background jobs: recurring catch-up (at start and daily after local midnight) and session sweeps.

use crate::auth::SESSION_TTL;
use crate::state::{AppState, today};
use chrono::{Duration, Local};
use jizifin_store::repo::{Recurring, Sessions};

/// Spawns the daily loop; the first run happens immediately.
pub fn spawn(state: AppState) {
    tokio::spawn(async move {
        loop {
            run_once(&state).await;
            tokio::time::sleep(until_next_midnight()).await;
        }
    });
}

/// Runs a single iteration of catch-up and session sweeping.
pub async fn run_once(state: &AppState) {
    let today = today();
    match state.write("entries.created", move |c| Recurring::materialize(c, today)).await {
        Ok(ids) => log_created(ids.len()),
        Err(e) => log_failure(&e.to_string()),
    }
    if let Err(e) = state.write_meta(|c| Sessions::sweep(c, SESSION_TTL)).await {
        log_failure(&e.to_string());
    }
}

fn until_next_midnight() -> std::time::Duration {
    let now = Local::now();
    let next = (now.date_naive() + Duration::days(1)).and_hms_opt(0, 0, 5).unwrap_or_default().and_local_timezone(Local).earliest().unwrap_or(now);
    (next - now).to_std().unwrap_or(std::time::Duration::from_secs(3600))
}

fn log_created(count: usize) {
    tracing::info!("recurring catch-up created {count} entries");
}

fn log_failure(detail: &str) {
    tracing::error!("scheduled job failed: {detail}");
}
