use std::sync::Arc;
use crate::{Context, Data, Error};
use poise::serenity_prelude::{ChannelId, MessageId, UserId};
use sqlx::{query, SqlitePool};

use serde_json;

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

pub fn cache_reminder(data: &Arc<Data>, r: &mut Reminder) -> () {
    let mut next_reminder = data.next_reminder.lock().unwrap();
    if let Some(stored_reminder) = &mut *next_reminder {
        if r.timestamp < stored_reminder.timestamp {
            *next_reminder = Some(r.clone());
        }
    } else {
        *next_reminder = Some(r.clone());
    }
}

pub async fn serialize_reminder(ctx: Context<'_>, r: &mut Reminder) -> Result<(), Error> {
    let users_json = serde_json::to_string(&r.user_ids).unwrap();
    let channel_id = r.channel_id.get() as i64;
    let message_id = r.message_id.get() as i64;
    let id = query!("INSERT INTO reminders (user_ids, message, timestamp, created_at, channel_id, message_id) VALUES (?, ?, ?, ?, ?, ?)",
        users_json, r.message, r.timestamp, r.created_at, channel_id, message_id).execute(&ctx.data().pool).await?.last_insert_rowid();
    r.id = Some(id as u32);

    cache_reminder(ctx.data(), r);
    Ok(())
}

pub async fn get_next_reminder(pool: &SqlitePool) -> Option<Reminder> {
    let next_reminder = query!("SELECT id, user_ids, message, timestamp, created_at, channel_id, message_id FROM reminders WHERE active = 1 ORDER BY timestamp ASC LIMIT 1").fetch_one(pool).await.ok();
    next_reminder.map(|next_reminder| {
        Reminder {
            id: Some(next_reminder.id as u32),
            timestamp: next_reminder.timestamp,
            created_at: next_reminder.created_at,
            user_ids: serde_json::from_str::<Vec<UserId>>(&next_reminder.user_ids).unwrap(),
            channel_id: ChannelId::new(next_reminder.channel_id as u64),
            message_id: MessageId::new(next_reminder.message_id as u64),
            message: next_reminder.message,
        }
    })
}