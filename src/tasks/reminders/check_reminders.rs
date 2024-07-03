use crate::commands::reminders::util::get_next_reminder;
use crate::{Data, BOT_COLOR, FALLBACK_CHANNEL, GUILD_ID};
use chrono::Utc;
use poise::serenity_prelude::{Context, CreateEmbed, CreateEmbedAuthor, CreateMessage};
use sqlx::query;
use std::fmt::Write;
use std::sync::Arc;
use tracing;

pub async fn check_reminders(ctx: &Context, data: &Arc<Data>) {
    let Some(reminder) = &mut data.next_reminder.lock().unwrap().clone() else {
        return;
    };
    if reminder.timestamp > Utc::now().timestamp() {
        return;
    };

    let embed = CreateEmbed::new().color(BOT_COLOR).author(
        CreateEmbedAuthor::new("Reminder notification!").icon_url(ctx.cache.current_user().face()),
    );
    let mut dm_disabled_users = Vec::new();

    for user_id in &reminder.user_ids {
        let username = match user_id.to_user(ctx).await {
            Ok(username) => username.name,
            Err(_) => continue,
        };
        let embed = embed.clone().description(format!(
            "Hey {0}! <t:{1}:R> on <t:{1}:F>, you asked me to remind you of {2}.\
            \n\n[View Message](https://hitori.discord.com/channels/{GUILD_ID}/{3}/{4})",
            username,
            reminder.timestamp,
            reminder.message,
            reminder.channel_id,
            reminder.message_id
        ));
        if user_id.direct_message(ctx, CreateMessage::new().embed(embed)).await.is_err() {
            dm_disabled_users.push(user_id);
        }
    }
    if !dm_disabled_users.is_empty() {
        let embed = embed.clone().description(format!(
            "Hey! <t:{0}:R> on <t:{0}:F>, you asked me to remind you of {1}.\
            \n\n[View Message](https://hitori.discord.com/channels/{GUILD_ID}/{2}/{3})",
            reminder.timestamp, reminder.message, reminder.channel_id, reminder.message_id
        ));
        let mut ping_content = String::new();
        for no_dm_user in dm_disabled_users {
            write!(ping_content, "<@{no_dm_user}> ").unwrap()
        }
        let _ = FALLBACK_CHANNEL
            .send_message(ctx, CreateMessage::new().embed(embed).content(ping_content))
            .await; // continue even if it can't send the message
    }

    if let Err(_) = query!("UPDATE reminders SET active = 0 WHERE id = ?", reminder.id)
        .execute(&data.pool)
        .await
    {
        tracing::warn!("{} failed to remove from database", reminder.id.unwrap());
    };

    let next_reminder = get_next_reminder(&data.pool).await;
    let mut stored_reminder = data.next_reminder.lock().unwrap();
    let Some(stored_reminder_data) = &mut *stored_reminder else {
        // Nothing in cache, replace with the next reminder or None
        *stored_reminder = next_reminder.clone();
        return;
    };
    if reminder.id == stored_reminder_data.id {
        // reminder that just finished was in the cache, replace it with the soonest one found in db
        *stored_reminder = next_reminder;
    } else {
        // Race condition happened, make sure the earliest reminder is next
        if let Some(next_reminder_data) = &next_reminder {
            if next_reminder_data.timestamp < stored_reminder_data.timestamp {
                *stored_reminder = next_reminder;
            }
        }
    }
}
