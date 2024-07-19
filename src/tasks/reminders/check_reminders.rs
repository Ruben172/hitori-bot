use crate::commands::reminders::util::{get_next_reminder_ts, user_ids_from_reminder_id};
use crate::{Data, BOT_COLOR, FALLBACK_CHANNEL, GUILD_ID};
use chrono::Utc;
use poise::serenity_prelude::{Context, CreateEmbed, CreateEmbedAuthor, CreateMessage};
use sqlx::query;
use std::fmt::Write;
use std::sync::Arc;

pub async fn check_reminders(ctx: &Context, data: &Arc<Data>) {
    let Some(next_timestamp) = data.next_reminder.lock().unwrap().clone() else {
        return;
    };
    if next_timestamp > Utc::now().timestamp() {
        return;
    };

    let embed = CreateEmbed::new().color(BOT_COLOR).author(
        CreateEmbedAuthor::new("Reminder notification!").icon_url(ctx.cache.current_user().face()),
    );
    let mut dm_disabled_users = Vec::new();
    
    let r = query!(
        r"SELECT r.id, message, timestamp, created_at, c.discord_id AS discord_channel_id, message_id 
        FROM reminders r
        JOIN reminder_channel rc ON rc.reminder_id = r.id
        JOIN channels c ON rc.channel_id = c.id
        WHERE active = 1 ORDER BY timestamp ASC LIMIT 1").fetch_one(&data.pool).await.unwrap(); // unwrap because tbh shit's joever if this fails
    let user_ids = user_ids_from_reminder_id(&data, r.id).await.unwrap(); 

    for user_id in user_ids {
        let username = match user_id.to_user(ctx).await {
            Ok(username) => username.name,
            Err(_) => continue,
        };
        let embed = embed.clone().description(format!(
            "Hey {0}! <t:{1}:R> on <t:{1}:F>, you asked me to remind you of {2}.\
            \n\n[View Message](https://hitori.discord.com/channels/{GUILD_ID}/{3}/{4})",
            username,
            r.timestamp,
            r.message,
            r.discord_channel_id,
            r.message_id
        ));
        if user_id.direct_message(ctx, CreateMessage::new().embed(embed)).await.is_err() {
            dm_disabled_users.push(user_id);
        }
    }
    if !dm_disabled_users.is_empty() {
        let embed = embed.clone().description(format!(
            "Hey! <t:{0}:R> on <t:{0}:F>, you asked me to remind you of {1}.\
            \n\n[View Message](https://hitori.discord.com/channels/{GUILD_ID}/{2}/{3})",
            r.timestamp, r.message, r.discord_channel_id, r.message_id
        ));
        let mut ping_content = String::new();
        for no_dm_user in dm_disabled_users {
            write!(ping_content, "<@{no_dm_user}> ").unwrap();
        }
        let _ = FALLBACK_CHANNEL
            .send_message(ctx, CreateMessage::new().embed(embed).content(ping_content))
            .await; // continue even if it can't send the message
    }

    if query!("UPDATE reminders SET active = 0 WHERE id = ?", r.id)
        .execute(&data.pool)
        .await
        .is_err()
    {
        tracing::warn!("{} failed to remove from database", r.id);
    };

    let upcoming_reminder = get_next_reminder_ts(&data.pool).await;
    let mut stored_reminder = data.next_reminder.lock().unwrap();
    let Some(stored_reminder_timestamp) = *stored_reminder else {
        // Nothing in cache, replace with the next reminder or None
        *stored_reminder = upcoming_reminder.clone();
        return;
    };
    if next_timestamp == stored_reminder_timestamp {
        // reminder that just finished was in the cache, replace it with the soonest one found in db
        *stored_reminder = upcoming_reminder;
    } else {
        // Race condition happened, make sure the earliest reminder is next
        if let Some(upcoming_reminder_timestamp) = upcoming_reminder {
            if upcoming_reminder_timestamp < stored_reminder_timestamp {
                *stored_reminder = upcoming_reminder;
            }
        }
    }
}
