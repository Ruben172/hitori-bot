use crate::util::send_ephemeral_text;
use crate::{Context, Data, Error};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use poise::serenity_prelude::{UserId};
use regex::Captures;
use sqlx::{query, SqlitePool};
use std::sync::Arc;

const MAX_REMINDERS: i32 = 25;
const NZ_TZ: Tz = chrono_tz::NZ;
const DAY_IN_SECONDS: i64 = 86400;

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
    year: i32, month: u32, day: u32, timezone: Tz,
) -> Result<i64, ()> {
    let Some(dt) = timezone.with_ymd_and_hms(year, month, day, 0, 0, 0).earliest() else {
        return Err(());
    };
    Ok(dt.with_timezone(&Utc).timestamp())
}

pub fn parse_ymd(
    data: &[Option<i32>], year_index: usize, day_index: usize,
) -> Result<(i32, u32, u32), ()> {
    let (Some(Some(mut year)), Some(Some(month)), Some(Some(day))) =
        (data.get(year_index), data.get(1), data.get(day_index))
    else {
        return Err(());
    };
    if year < 100 {
        year += 2000;
    }
    Ok((year, *month as u32, *day as u32))
}

pub fn parse_naivetime(data: &[Option<i32>], hour_index: usize) -> Result<NaiveTime, ()> {
    let (Some(Some(hours)), Some(Some(minutes)), Some(seconds)) =
        (data.get(hour_index), data.get(hour_index + 1), data.get(hour_index + 2))
    else {
        return Err(());
    };
    let seconds = seconds.unwrap_or(0);
    let Some(time) = NaiveTime::from_hms_opt(*hours as u32, *minutes as u32, seconds as u32) else {
        return Err(());
    };
    Ok(time)
}

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
                let (year, month, day) = parse_ymd(&int_matches, 0, 2)?;
                return date_timezone_to_timestamp(year, month, day, NZ_TZ);
            } else if let Some(captures) = &rc.date_dmy.captures(timestamp) {
                let Ok(int_matches) = matches_to_vecint(captures) else { return Err(()) };
                let (year, month, day) = parse_ymd(&int_matches, 2, 0)?;
                return date_timezone_to_timestamp(year, month, day, NZ_TZ);
            } else if let Some(captures) = &rc.time.captures(timestamp) {
                let Ok(int_matches) = matches_to_vecint(captures) else { return Err(()) };
                let time = parse_naivetime(&int_matches, 0)?;
                let date = Utc::now().date_naive();
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
                let (year, month, day) = parse_ymd(&int_matches, 0, 2)?;
                let Some(date) = NaiveDate::from_ymd_opt(year, month, day) else {
                    return Err(());
                };
                let time = parse_naivetime(&int_matches, 3)?;
                return Ok(NaiveDateTime::new(date, time).and_utc().timestamp());
            } else if let Some(captures) = &rc.datetime_dmy.captures(timestamp) {
                let Ok(int_matches) = matches_to_vecint(captures) else { return Err(()) };
                let (year, month, day) = parse_ymd(&int_matches, 2, 0)?;
                let Some(date) = NaiveDate::from_ymd_opt(year, month, day) else {
                    return Err(());
                };
                let time = parse_naivetime(&int_matches, 3)?;
                return Ok(NaiveDateTime::new(date, time).and_utc().timestamp());
            }
            Err(())
        }
        _ => Err(()),
    }
}

pub fn cache_reminder(data: &Arc<Data>, r: i64) {
    let mut next_reminder = data.next_reminder.lock().unwrap();
    if let Some(stored_reminder) = *next_reminder {
        if r < stored_reminder {
            *next_reminder = Some(r.clone());
        }
    } else {
        *next_reminder = Some(r.clone());
    }
}

pub async fn get_next_reminder_ts(pool: &SqlitePool) -> Option<i64> {
    let next_reminder = query!("SELECT timestamp FROM reminders WHERE active = 1 ORDER BY timestamp ASC LIMIT 1").fetch_one(pool).await.ok();
    next_reminder.map(|x| x.timestamp)
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
    .await
    .ok();

    let Some(reminder) = reminder else {
        // send_ephemeral_text(ctx, "Reminder does not exist or has already expired.").await?;
        return Err("".into());
    };

    Ok(reminder.into_iter().map(|x| UserId::new(x.discord_id as u64)).collect::<Vec<UserId>>())
}

pub async fn check_author_reminder_count(ctx: Context<'_>) -> Result<(), Error> {
    let author_id = ctx.author().id.get() as i64;
    let reminder_count = query!(
        r"SELECT COUNT(*) AS count 
        FROM reminders r 
        JOIN reminder_user ru ON r.id = ru.reminder_id 
        JOIN users u on ru.user_id = u.id 
        WHERE u.discord_id = ?",
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
