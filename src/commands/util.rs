use crate::{Context, Data, Error};
use poise::serenity_prelude::{ChannelId, Message, MessageId, UserId};
use regex::Captures;
use std::sync::Arc;
use sqlx::{query, query_scalar};

pub fn message_id_from_ctx(ctx: Context<'_>) -> MessageId {
    match ctx {
        Context::Application(actx) => actx.interaction.id.get().into(),
        Context::Prefix(pctx) => pctx.msg.id,
    }
}

pub fn referenced_from_ctx(ctx: Context<'_>) -> Option<Message> {
    match ctx {
        Context::Application(_actx) => None,
        Context::Prefix(pctx) => pctx.msg.referenced_message.as_ref().map(|m| *m.clone()),
    }
}

pub async fn ensure_user_in_db(data: &Arc<Data>, user: UserId) -> Result<(), Error> {
    let author_id = user.get() as i64;
    query!(r"INSERT OR IGNORE INTO users (discord_id) VALUES (?)", author_id)
        .execute(&data.pool)
        .await?;
    Ok(())
}

pub async fn get_internal_user_id(data: &Arc<Data>, user: UserId) -> Result<i64, Error> {
    let author_id = user.get() as i64;
    ensure_user_in_db(data, user).await?;
    query_scalar!(r"SELECT id FROM users WHERE discord_id = ?", author_id)
        .fetch_one(&data.pool)
        .await?
        .ok_or("".into())
}

pub async fn ensure_channel_in_db(data: &Arc<Data>, channel: ChannelId) -> Result<(), Error> {
    let channel_id = channel.get() as i64;
    query!(r"INSERT OR IGNORE INTO users (discord_id) VALUES (?)", channel_id)
        .execute(&data.pool)
        .await?;
    Ok(())
}

pub async fn get_internal_channel_id(data: &Arc<Data>, channel: ChannelId) -> Result<i64, Error> {
    let channel_id = channel.get() as i64;
    ensure_channel_in_db(data, channel).await?;
    query_scalar!(r"SELECT id FROM channels WHERE discord_id = ?", channel_id)
        .fetch_one(&data.pool)
        .await?
        .ok_or("".into())
}

pub fn matches_to_vecint(captures: &Captures) -> Result<Vec<Option<i32>>, Error> {
    let mut int_matches = Vec::new();
    for capture in captures.iter().skip(1) {
        match capture {
            Some(c) => {
                let Ok(parsed_amount) = c.as_str().parse::<i32>() else {
                    return Err("Failed to parse arguments".into());
                };
                int_matches.push(Some(parsed_amount));
            }
            None => int_matches.push(None),
        }
    }
    Ok(int_matches)
}

pub fn multiply_by_position(data: &[Option<i32>], table: &[i32]) -> Result<i32, Error> {
    let mut amount: i32 = 0;

    for (i, capture) in data.iter().enumerate() {
        let Some(c) = capture else {
            continue;
        };
        let Some(rhs) = table.get(i) else {
            return Err("Failed to parse arguments".into());
        };
        let Some(multiplied_amount) = c.checked_mul(*rhs) else {
            return Err("Failed to parse arguments".into());
        };
        if amount.checked_add(multiplied_amount).is_none() {
            return Err("Failed to parse arguments".into());
        };
        amount += multiplied_amount;
    }
    Ok(amount)
}

pub fn parse_utc_offset(data: &Arc<Data>, offset: &str) -> Result<i32, Error> {
    let regex = &data.regex_cache.utc_offset;
    let Some(captures) = &regex.captures(offset) else { return Err("Invalid offset.".into()) };
    let matches = matches_to_vecint(captures)?;
    let sign;
    if let Some(Some(signed)) = matches.first() {
        sign = signed.signum();
    } else {
        sign = 1;
    }
    let minute_conversions: [i32; 2] = [60, sign]; // first number will always be signed, second number should be multiplied by 1 and the sign
    let minutes = multiply_by_position(&matches, &minute_conversions)?;
    Ok(minutes)
}