use crate::util::send_ephemeral_text;
use crate::{Context, Data, Error};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use poise::serenity_prelude::{ChannelId, MessageId, UserId};
use regex::Captures;
use serde_json;
use sqlx::{query, SqlitePool};
use std::sync::Arc;

const MAX_REMINDERS: i32 = 25;
const NZ_TZ: Tz = chrono_tz::NZ;
const DAY_IN_SECONDS: i64 = 86400;

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

fn matches_to_vecint(captures: &Captures) -> Result<Vec<Option<i32>>, ()> {
    let mut int_matches = Vec::new();
    for capture in captures.iter().skip(1) {
        match capture {
            Some(c) => {
                let Ok(parsed_amount) = c.as_str().parse::<i32>() else {
                    return Err(());
                };
                int_matches.push(Some(parsed_amount));
            }
            None => int_matches.push(None),
        }
    }
    Ok(int_matches)
}

fn match_to_int(captures: &Captures) -> Result<i32, ()> {
    let Some(capture) = captures.get(1) else { return Err(()) };
    let Ok(parsed_amount) = capture.as_str().parse::<i32>() else {
        return Err(());
    };
    Ok(parsed_amount)
}

fn multiply_by_position(data: &[Option<i32>], table: &[i32]) -> Result<i32, ()> {
    let mut amount: i32 = 0;

    for (i, capture) in data.iter().enumerate() {
        let Some(c) = capture else {
            continue;
        };
        let Some(rhs) = table.get(i) else {
            return Err(());
        };
        let Some(multiplied_amt) = c.checked_mul(*rhs) else {
            return Err(());
        };
        if amount.checked_add(multiplied_amt).is_none() {
            return Err(());
        };
        amount += multiplied_amt;
    }
    Ok(amount)
}

pub fn date_timezone_to_timestamp(
    mut year: i32, month: u32, day: u32, timezone: Tz,
) -> Result<i64, ()> {
    if year < 100 {
        year += 2000;
    }
    let Some(dt) = timezone.with_ymd_and_hms(year, month, day, 0, 0, 0).earliest() else {
        return Err(());
    };
    Ok(dt.with_timezone(&Utc).timestamp())
}

#[allow(clippy::too_many_lines)] // TODO?
pub fn parse_timestamp(data: &Arc<Data>, timestamp: &str) -> Result<i64, ()> {
    let rc = &data.regex_cache;
    match timestamp.split_whitespace().count() {
        1 => {
            if let Some(captures) = &rc.relative_time.captures(timestamp) {
                let second_conversions: [i32; 7] = [31557600, 2629800, 604800, 86400, 3600, 60, 1]; // year, month, week, day, hour, minute, second
                let seconds =
                    multiply_by_position(&matches_to_vecint(captures)?, &second_conversions)?;
                return Ok(Utc::now().timestamp() + seconds as i64);
            } else if let Some(captures) = &rc.date_ymd.captures(timestamp) {
                let Ok(int_matches) = matches_to_vecint(captures) else { return Err(()) };
                let (Some(Some(year)), Some(Some(month)), Some(Some(date))) =
                    (int_matches.get(0), int_matches.get(1), int_matches.get(2))
                else {
                    return Err(());
                };
                return date_timezone_to_timestamp(*year, *month as u32, *date as u32, NZ_TZ);
            } else if let Some(captures) = &rc.date_dmy.captures(timestamp) {
                let Ok(int_matches) = matches_to_vecint(captures) else { return Err(()) };
                let (Some(Some(date)), Some(Some(month)), Some(Some(year))) =
                    (int_matches.get(0), int_matches.get(1), int_matches.get(2))
                else {
                    return Err(());
                };
                return date_timezone_to_timestamp(*year, *month as u32, *date as u32, NZ_TZ);
            } else if let Some(captures) = &rc.time.captures(timestamp) {
                let Ok(int_matches) = matches_to_vecint(captures) else { return Err(()) };
                let (Some(Some(hours)), Some(Some(minutes)), Some(seconds)) =
                    (int_matches.get(0), int_matches.get(1), int_matches.get(2))
                else {
                    return Err(());
                };
                let seconds = seconds.unwrap_or(0);
                let date = Utc::now().date_naive();
                let Some(time) =
                    NaiveTime::from_hms_opt(*hours as u32, *minutes as u32, seconds as u32)
                else {
                    return Err(());
                };
                let timestamp = NaiveDateTime::new(date, time).and_utc().timestamp();
                if timestamp < Utc::now().timestamp() {
                    return Ok(timestamp + DAY_IN_SECONDS);
                }
                return Ok(timestamp);
            } else if let Some(captures) = &rc.relative_minutes.captures(timestamp) {
                let minutes = match_to_int(captures)?;
                let Some(seconds) = minutes.checked_mul(60) else {
                    return Err(());
                };
                return Ok(Utc::now().timestamp() + seconds as i64);
            } else if let Some(captures) = &rc.unix_timestamp.captures(timestamp) {
                return match_to_int(captures).map(|t| t as i64);
            }
            Err(())
        }
        2 => {
            if let Some(captures) = &rc.datetime_ymd.captures(timestamp) {
                let Ok(int_matches) = matches_to_vecint(captures) else { return Err(()) };
                let (
                    Some(Some(year)),
                    Some(Some(month)),
                    Some(Some(day)),
                    Some(Some(hours)),
                    Some(Some(minutes)),
                    Some(seconds),
                ) = (
                    int_matches.get(0),
                    int_matches.get(1),
                    int_matches.get(2),
                    int_matches.get(3),
                    int_matches.get(4),
                    int_matches.get(5),
                )
                else {
                    return Err(());
                };
                let seconds = seconds.unwrap_or_else(|| 0);
                let Some(date) = NaiveDate::from_ymd_opt(*year, *month as u32, *day as u32) else {
                    return Err(());
                };
                let Some(time) =
                    NaiveTime::from_hms_opt(*hours as u32, *minutes as u32, seconds as u32)
                else {
                    return Err(());
                };
                return Ok(NaiveDateTime::new(date, time).and_utc().timestamp());
            } else if let Some(captures) = &rc.datetime_dmy.captures(timestamp) {
                let Ok(int_matches) = matches_to_vecint(captures) else { return Err(()) };
                let (
                    Some(Some(day)),
                    Some(Some(month)),
                    Some(Some(mut year)),
                    Some(Some(hours)),
                    Some(Some(minutes)),
                    Some(seconds),
                ) = (
                    int_matches.get(0),
                    int_matches.get(1),
                    int_matches.get(2),
                    int_matches.get(3),
                    int_matches.get(4),
                    int_matches.get(5),
                )
                else {
                    return Err(());
                };
                if year < 100 {
                    year += 2000;
                }
                let seconds = seconds.unwrap_or(0);
                let Some(date) = NaiveDate::from_ymd_opt(year, *month as u32, *day as u32) else {
                    return Err(());
                };
                let Some(time) =
                    NaiveTime::from_hms_opt(*hours as u32, *minutes as u32, seconds as u32)
                else {
                    return Err(());
                };
                return Ok(NaiveDateTime::new(date, time).and_utc().timestamp());
            } 
            Err(())
        }
        _ => Err(())
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
    serde_json::from_str::<Vec<UserId>>(users_str).unwrap()
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
