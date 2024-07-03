use crate::commands::reminders::util::{cache_reminder, get_next_reminder};
use crate::{Data, Error};
use poise::serenity_prelude::Context;
use reminders::check_reminders;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

mod reminders;

pub async fn task_handler(ctx: Context, data: Arc<Data>) -> Result<(), Error> {
    let mut reminder_interval = interval(Duration::from_secs(5));
    if let Some(mut reminder) = get_next_reminder(&data.pool).await {
        cache_reminder(&data, &mut reminder);
    }
    loop {
        reminder_interval.tick().await;

        check_reminders(&ctx, &data).await;
    }
}
