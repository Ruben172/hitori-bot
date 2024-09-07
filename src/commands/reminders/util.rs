use crate::{Context, Data, Error};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use poise::serenity_prelude::UserId;
use regex::Captures;
use sqlx::{query, query_scalar, SqlitePool};
use std::sync::Arc;
use crate::commands::util::{matches_to_vecint, multiply_by_position};

const MAX_REMINDERS: i64 = 25;
const DAY_IN_SECONDS: i64 = 86400;

fn match_to_int(captures: &Captures) -> Result<i32, Error> {
    let Some(capture) = captures.get(1) else { return Err("Ah, um... it looks like the timestamp is invalid... I-I don't really understand it. C-could you maybe check it again?".into()) };
    let Ok(parsed_amount) = capture.as_str().parse::<i32>() else {
        return Err("Ah, um... it looks like the timestamp is invalid... I-I don't really understand it. C-could you maybe check it again?".into());
    };
    Ok(parsed_amount)
}

pub fn date_to_timestamp(
    year: i32, month: u32, day: u32,
) -> Result<i64, Error> {
    let Some(dt) = Utc.with_ymd_and_hms(year, month, day, 0, 0, 0).earliest() else {
        return Err("Ah, um... it looks like the timestamp is invalid... I-I don't really understand it. C-could you maybe check it again?".into());
    };
    Ok(dt.timestamp())
}

pub fn parse_ymd(
    data: &[Option<i32>], year_index: usize, day_index: usize,
) -> Result<(i32, u32, u32), Error> {
    let (Some(Some(mut year)), Some(Some(month)), Some(Some(day))) =
        (data.get(year_index), data.get(1), data.get(day_index))
    else {
        return Err("Ah, um... it looks like the timestamp is invalid... I-I don't really understand it. C-could you maybe check it again?".into());
    };
    if year < 100 {
        year += 2000;
    }
    Ok((year, *month as u32, *day as u32))
}

pub fn parse_naivetime(data: &[Option<i32>], hour_index: usize) -> Result<NaiveTime, Error> {
    let (Some(Some(hours)), Some(Some(minutes)), Some(seconds)) =
        (data.get(hour_index), data.get(hour_index + 1), data.get(hour_index + 2))
    else {
        return Err("Ah, um... it looks like the timestamp is invalid... I-I don't really understand it. C-could you maybe check it again?".into());
    };
    let seconds = seconds.unwrap_or(0);
    let Some(time) = NaiveTime::from_hms_opt(*hours as u32, *minutes as u32, seconds as u32) else {
        return Err("Ah, um... it looks like the timestamp is invalid... I-I don't really understand it. C-could you maybe check it again?".into());
    };
    Ok(time)
}

pub fn parse_timestamp(data: &Arc<Data>, timestamp: &str, offset: i64) -> Result<i64, Error> {
    let rc = &data.regex_cache;
    match timestamp.split_whitespace().count() {
        1 => {
            if let Some(captures) = &rc.relative_time.captures(timestamp) {
                let second_conversions: [i32; 7] = [31557600, 2629800, 604800, 86400, 3600, 60, 1]; // year, month, week, day, hour, minute, second
                let seconds =
                    multiply_by_position(&matches_to_vecint(captures)?, &second_conversions)?;
                return Ok(Utc::now().timestamp() + seconds as i64);
            } else if let Some(captures) = &rc.date_ymd.captures(timestamp) {
                let Ok(int_matches) = matches_to_vecint(captures) else { return Err("Ah, um... it looks like the timestamp is invalid... I-I don't really understand it. C-could you maybe check it again?".into()) };
                let (year, month, day) = parse_ymd(&int_matches, 0, 2)?;
                return Ok(date_to_timestamp(year, month, day)? - offset * 60);
            } else if let Some(captures) = &rc.date_dmy.captures(timestamp) {
                let Ok(int_matches) = matches_to_vecint(captures) else { return Err("Ah, um... it looks like the timestamp is invalid... I-I don't really understand it. C-could you maybe check it again?".into()) };
                let (year, month, day) = parse_ymd(&int_matches, 2, 0)?;
                return Ok(date_to_timestamp(year, month, day)? - offset * 60);
            } else if let Some(captures) = &rc.time.captures(timestamp) {
                let Ok(int_matches) = matches_to_vecint(captures) else { return Err("Ah, um... it looks like the timestamp is invalid... I-I don't really understand it. C-could you maybe check it again?".into()) };
                let time = parse_naivetime(&int_matches, 0)?;
                let date = Utc::now().date_naive();
                let timestamp = NaiveDateTime::new(date, time).and_utc().timestamp() - offset * 60;
                if timestamp < Utc::now().timestamp() {
                    return Ok(timestamp + DAY_IN_SECONDS);
                }
                return Ok(timestamp);
            } else if let Some(captures) = &rc.relative_minutes.captures(timestamp) {
                let minutes = match_to_int(captures)?;
                let Some(seconds) = minutes.checked_mul(60) else {
                    return Err("U-um... I'm really sorry, but... this reminder duration is... uh... too long! I-I might forget it, so... could we maybe shorten it? If that's okay with you...?".into());
                };
                return Ok(Utc::now().timestamp() + seconds as i64);
            } else if let Some(captures) = &rc.unix_timestamp.captures(timestamp) {
                return match_to_int(captures).map(|t| t as i64);
            }
            Err("Ah, um... it looks like the timestamp is invalid... I-I don't really understand it. C-could you maybe check it again?".into())
        }
        2 => {
            if let Some(captures) = &rc.datetime_ymd.captures(timestamp) {
                let Ok(int_matches) = matches_to_vecint(captures) else { return Err("Ah, um... it looks like the timestamp is invalid... I-I don't really understand it. C-could you maybe check it again?".into()) };
                let (year, month, day) = parse_ymd(&int_matches, 0, 2)?;
                let Some(date) = NaiveDate::from_ymd_opt(year, month, day) else {
                    return Err("Ah, um... it looks like the timestamp is invalid... I-I don't really understand it. C-could you maybe check it again?".into());
                };
                let time = parse_naivetime(&int_matches, 3)?;
                return Ok(NaiveDateTime::new(date, time).and_utc().timestamp() - offset * 60);
            } else if let Some(captures) = &rc.datetime_dmy.captures(timestamp) {
                let Ok(int_matches) = matches_to_vecint(captures) else { return Err("Ah, um... it looks like the timestamp is invalid... I-I don't really understand it. C-could you maybe check it again?".into()) };
                let (year, month, day) = parse_ymd(&int_matches, 2, 0)?;
                let Some(date) = NaiveDate::from_ymd_opt(year, month, day) else {
                    return Err("Ah, um... it looks like the timestamp is invalid... I-I don't really understand it. C-could you maybe check it again?".into());
                };
                let time = parse_naivetime(&int_matches, 3)?;
                return Ok(NaiveDateTime::new(date, time).and_utc().timestamp() - offset * 60);
            }
            Err("Ah, um... it looks like the timestamp is invalid... I-I don't really understand it. C-could you maybe check it again?".into())
        }
        _ => Err("Um, it seems there are too many arguments for the timestamp... I-I'm having trouble understanding it. C-could you simplify it a bit?".into()),
    }
}

pub fn cache_reminder(data: &Arc<Data>, r: i64) {
    let mut next_reminder = data.next_reminder.lock().unwrap();
    if let Some(stored_reminder) = *next_reminder {
        if r < stored_reminder {
            *next_reminder = Some(r);
        }
    } else {
        *next_reminder = Some(r);
    }
}

pub async fn get_next_reminder_ts(pool: &SqlitePool) -> Option<i64> {
    let next_reminder =
        query!("SELECT timestamp FROM reminders WHERE active = 1 ORDER BY timestamp ASC LIMIT 1")
            .fetch_one(pool)
            .await
            .ok();
    next_reminder.map(|x| x.timestamp)
}

pub async fn reminder_exists_and_active(data: &Arc<Data>, reminder_id: i64) -> bool {
    let Ok(exists) = query_scalar!(r"SELECT EXISTS(SELECT 1 FROM reminders WHERE id = ? AND active = 1)", reminder_id).fetch_one(&data.pool).await else {
        return false
    };
    exists != 0
}

pub async fn user_ids_from_reminder_id(
    data: &Arc<Data>, reminder_id: i64,
) -> Result<Vec<UserId>, Error> {
    let reminder = query!(
        r"SELECT discord_id 
        FROM users u 
        JOIN reminder_user ru ON ru.user_id = u.id
        JOIN reminders r ON ru.reminder_id = r.id
        WHERE r.id = ? AND active = 1",
        reminder_id
    )
    .fetch_all(&data.pool)
    .await;

    let Ok(reminder) = reminder else {
        return Err("Error fetching users".into());
    };
    
    Ok(reminder.into_iter().map(|x| UserId::new(x.discord_id as u64)).collect::<Vec<UserId>>())
}

pub async fn guild_from_reminder_id(
    data: &Arc<Data>, reminder_id: i64,
) -> Result<i64, Error> {
    let reminder = query!(
        r"SELECT discord_id 
        FROM guilds g
        JOIN reminder_guild rg ON rg.guild_id = g.id
        JOIN reminders r ON rg.reminder_id = r.id
        WHERE r.id = ? AND active = 1",
        reminder_id
    )
    .fetch_one(&data.pool)
    .await;

    let Ok(reminder) = reminder else {
        return Err("Error fetching guild".into());
    };
    
    Ok(reminder.discord_id)
}

pub async fn check_author_reminder_count(ctx: Context<'_>) -> Result<bool, Error> {
    let author_id = ctx.author().id.get() as i64;
    let reminder_count = query!(
        r"SELECT COUNT(*) AS count 
        FROM reminders r 
        JOIN reminder_user ru ON r.id = ru.reminder_id 
        JOIN users u on ru.user_id = u.id 
        WHERE u.discord_id = ? AND active = 1",
        author_id
    )
    .fetch_one(&ctx.data().pool)
    .await?
    .count;
    if reminder_count >= MAX_REMINDERS {
        return Err("Ah, um, you have too many active reminders... I-I'm afraid I can't add any more right now.".into());
    }
    Ok(true)
}

