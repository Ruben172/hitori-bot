use crate::commands::reminders::util::{get_next_reminder_ts, user_ids_from_reminder_id};
use crate::{Data, BOT_COLOR};
use chrono::Utc;
use poise::serenity_prelude::{ChannelId, Context, CreateEmbed, CreateEmbedAuthor, CreateMessage};
use sqlx::query;
use std::fmt::Write;
use std::sync::Arc;
use crate::util::url_guild_id;

pub async fn check_reminders(ctx: &Context, data: &Arc<Data>) {
    let Some(next_timestamp) = *data.next_reminder.lock().unwrap() else {
        return;
    };
    if next_timestamp > Utc::now().timestamp() {
        return;
    };

    let embed = CreateEmbed::new().color(BOT_COLOR).author(
        CreateEmbedAuthor::new("Reminder notification!").icon_url(ctx.cache.current_user().face()),
    );
    let mut dm_disabled_users = Vec::new();

    let r = query!( // First upcoming reminder
        r"SELECT r.id, message, timestamp, created_at, c.discord_id AS channel_id, g.discord_id AS guild_id, message_id, fc.discord_id AS fallback_channel
        FROM reminders r
        JOIN reminder_channel rc ON rc.reminder_id = r.id JOIN channels c ON rc.channel_id = c.id
        JOIN reminder_guild rg ON r.id = rg.reminder_id JOIN guilds g ON rg.guild_id = g.id
        LEFT JOIN channels fc ON fc.id = g.fallback_channel 
        WHERE active = 1 ORDER BY timestamp ASC LIMIT 1").fetch_one(&data.pool).await.unwrap(); // unwrap because tbh shit's joever if this fails
    let user_ids = user_ids_from_reminder_id(data, r.id).await.unwrap();

    for user_id in user_ids {
        let username = match user_id.to_user(ctx).await {
            Ok(username) => username.name,
            Err(_) => continue,
        };
        let embed = embed.clone().description(format!(
            "Hey {0}! <t:{1}:R> on <t:{1}:F>, you asked me to remind you of {2}.\
            \n\n[View Message](https://hitori.discord.com/channels/{3}/{4}/{5})",
            username, r.timestamp, r.message, url_guild_id(r.guild_id), r.channel_id, r.message_id
        ));
        if user_id.direct_message(ctx, CreateMessage::new().embed(embed)).await.is_err() {
            dm_disabled_users.push(user_id);
        }
    }
    if !dm_disabled_users.is_empty() && r.fallback_channel.is_some() {
        let fallback_channel = ChannelId::new(r.fallback_channel.unwrap() as u64);
        let embed = embed.clone().description(format!(
            "Hey! <t:{0}:R> on <t:{0}:F>, you asked me to remind you of {1}.\
            \n\n[View Message](https://hitori.discord.com/channels/{2}/{3}/{4})",
            r.timestamp, r.message, url_guild_id(r.guild_id), r.channel_id, r.message_id
        ));
        let mut ping_content = String::new();
        for no_dm_user in dm_disabled_users {
            write!(ping_content, "<@{no_dm_user}> ").unwrap();
        }
        let _ = fallback_channel
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
        *stored_reminder = upcoming_reminder;
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
