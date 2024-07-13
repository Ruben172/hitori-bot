use crate::{Context, Data, Error};
use poise::serenity_prelude::{ChannelId, MessageId, UserId};
use sqlx::{query, SqlitePool};
use std::sync::Arc;
use chrono::Utc;
use regex::Captures;
use crate::util::send_ephemeral_text;
use serde_json;

const MAX_REMINDERS: i32 = 25;

#[derive(Debug, Clone)]
pub struct Reminder {
    pub id: Option<u32>,
    pub timestamp: i64,
    pub created_at: i64,
    pub user_ids: Vec<UserId>,
    pub channel_id: ChannelId,
    pub message_id: MessageId,
    pub message: String,
}

fn matches_to_vecint(captures: Captures) -> Result<Vec<Option<i32>>, ()> {
    let mut int_matches = Vec::new();
    for capture in captures.iter().skip(1) {
        match capture {
            Some(c) => {
                let Ok(parsed_amount) = c.as_str().parse::<i32>() else {
                    return Err(());
                };
                int_matches.push(Some(parsed_amount))
            }
            None => int_matches.push(None),
        }
    }
    Ok(int_matches)
}

fn relative_matches_to_seconds(captures: Captures) -> Result<i32, ()> {
    let second_conversions: [i32; 7] = [31557600, 2629800, 604800, 86400, 3600, 60, 1]; // year, month, week, day, hour, minute, second
    let mut seconds: i32 = 0;
    for (i, c) in captures.iter().skip(1).enumerate() {
        let Some(c) = c else {
            continue;
        };
        let Ok(parsed_length) = c.as_str().parse::<i32>() else {
            return Err(());
        };
        let Some(parsed_seconds) = parsed_length.checked_mul(second_conversions[i]) else {
            return Err(());
        };
        if seconds.checked_add(parsed_seconds).is_none() {
            return Err(());
        };
        seconds += parsed_seconds;
    }
    if seconds > 34560000 {
        return Err(());
    };
    Ok(seconds)
}

pub fn parse_timestamp(data: &Arc<Data>, timestamp: String) -> Result<i64, ()> {
    let regex_cache = &data.regex_cache;
    let relative_time = &regex_cache.relative_time;

    match timestamp.split_whitespace().count() {
        1 => {
            let Some(captures) = relative_time.captures(&timestamp) else {
                return Err(());
            };
            let seconds = relative_matches_to_seconds(captures)?;
            Ok(Utc::now().timestamp() + seconds as i64)
        }
        // 2 => {
        //
        // }
        _ => return Err(())
    }
}

pub fn cache_reminder(data: &Arc<Data>, r: &mut Reminder) {
    let mut next_reminder = data.next_reminder.lock().unwrap();
    if let Some(stored_reminder) = &mut *next_reminder {
        if r.timestamp < stored_reminder.timestamp {
            *next_reminder = Some(r.clone());
        }
    } else {
        *next_reminder = Some(r.clone());
    }
}

pub async fn get_next_reminder(pool: &SqlitePool) -> Option<Reminder> {
    let next_reminder = query!("SELECT id, user_ids, message, timestamp, created_at, channel_id, message_id FROM reminders WHERE active = 1 ORDER BY timestamp ASC LIMIT 1").fetch_one(pool).await.ok();
    next_reminder.map(|next_reminder| Reminder {
        id: Some(next_reminder.id as u32),
        timestamp: next_reminder.timestamp,
        created_at: next_reminder.created_at,
        user_ids: deserialize_user_ids(&next_reminder.user_ids),
        channel_id: ChannelId::new(next_reminder.channel_id as u64),
        message_id: MessageId::new(next_reminder.message_id as u64),
        message: next_reminder.message,
    })
}

pub async fn serialize_reminder(ctx: Context<'_>, r: &mut Reminder) -> Result<(), Error> {
    let users_json = serialize_user_ids(&r.user_ids);
    let channel_id = r.channel_id.get() as i64;
    let message_id = r.message_id.get() as i64;
    let id = query!("INSERT INTO reminders (user_ids, message, timestamp, created_at, channel_id, message_id) VALUES (?, ?, ?, ?, ?, ?)",
        users_json, r.message, r.timestamp, r.created_at, channel_id, message_id).execute(&ctx.data().pool).await?.last_insert_rowid();
    r.id = Some(id as u32);

    cache_reminder(ctx.data(), r);
    Ok(())
}

pub fn serialize_user_ids(user_ids: &Vec<UserId>) -> String {
    serde_json::to_string(&user_ids).unwrap()
}

pub fn deserialize_user_ids(users_str: &str) -> Vec<UserId> {
    serde_json::from_str::<Vec<UserId>>(&users_str).unwrap()
}

pub async fn user_ids_from_reminder_id(
    ctx: Context<'_>, reminder_id: u32,
) -> Result<Vec<UserId>, Error> {
    let reminder =
        query!("SELECT user_ids FROM reminders WHERE id = ? AND active = 1", reminder_id)
            .fetch_one(&ctx.data().pool)
            .await
            .ok();

    let Some(reminder) = reminder else {
        send_ephemeral_text(ctx, "Reminder does not exist or has already expired.").await?;
        return Err("".into());
    };

    Ok(deserialize_user_ids(&reminder.user_ids))
}

pub async fn check_author_reminder_count(ctx: Context<'_>) -> Result<(), Error> {
    let author_id = ctx.author().id.get() as i64;
    let reminder_count = query!(
        r"SELECT COUNT(*) AS count FROM reminders WHERE user_ids LIKE '%'||?||'%' AND active = 1",
        author_id
    )
    .fetch_one(&ctx.data().pool)
    .await?
    .count;
    if reminder_count >= MAX_REMINDERS {
        send_ephemeral_text(ctx, "You have too many active reminders").await?;
        return Err("".into());
    }
    Ok(())
}
