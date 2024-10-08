use crate::{Context, Data, Error};
use poise::serenity_prelude::{ChannelId, GuildId, Message, MessageId, UserId};
use regex::Captures;
use sqlx::{query, query_scalar};
use std::sync::Arc;

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
    if query_scalar!(r"SELECT COUNT(1) FROM users WHERE (discord_id) = (?)", author_id)
        .fetch_one(&data.pool)
        .await?
        .eq(&1)
    {
        return Ok(());
    }
    query!(r"INSERT OR IGNORE INTO users (discord_id) VALUES (?)", author_id)
        .execute(&data.pool)
        .await?;
    Ok(())
}

pub async fn get_internal_user_id(data: &Arc<Data>, user: UserId) -> Result<i64, Error> {
    let author_id = user.get() as i64;
    ensure_user_in_db(data, user).await?;
    Ok(query_scalar!(r"SELECT id FROM users WHERE discord_id = ?", author_id)
        .fetch_one(&data.pool)
        .await?)
}

pub async fn ensure_channel_in_db(data: &Arc<Data>, channel: ChannelId) -> Result<(), Error> {
    let channel_id = channel.get() as i64;
    if query_scalar!(r"SELECT COUNT(1) FROM channels WHERE (discord_id) = (?)", channel_id)
        .fetch_one(&data.pool)
        .await?
        .eq(&1)
    {
        return Ok(());
    }
    query!(r"INSERT OR IGNORE INTO channels (discord_id) VALUES (?)", channel_id)
        .execute(&data.pool)
        .await?;
    Ok(())
}

pub async fn get_internal_channel_id(data: &Arc<Data>, channel: ChannelId) -> Result<i64, Error> {
    let channel_id = channel.get() as i64;
    ensure_channel_in_db(data, channel).await?;
    Ok(query_scalar!(r"SELECT id FROM channels WHERE discord_id = ?", channel_id)
        .fetch_one(&data.pool)
        .await?)
}

pub async fn ensure_guild_in_db(ctx: Context<'_>, guild: Option<GuildId>) -> Result<(), Error> {
    let guild_id = force_guild_id(guild);
    if query_scalar!(r"SELECT COUNT(1) FROM guilds WHERE (discord_id) = (?)", guild_id)
        .fetch_one(&ctx.data().pool)
        .await?
        .eq(&1)
    {
        return Ok(());
    }
    let i_fallback_channel_id = if guild.is_some() {
        Some(get_internal_channel_id(ctx.data(), ctx.channel_id()).await?)
    } else {
        None // DMs should not have a fallback channel
    };
    query!(
        r"INSERT OR IGNORE INTO guilds (discord_id, fallback_channel) VALUES (?, ?)",
        guild_id,
        i_fallback_channel_id
    )
    .execute(&ctx.data().pool)
    .await?;
    Ok(())
}

pub async fn get_internal_guild_id(ctx: Context<'_>, guild: Option<GuildId>) -> Result<i64, Error> {
    let guild_id = force_guild_id(guild);
    ensure_guild_in_db(ctx, guild).await?;
    Ok(query_scalar!(r"SELECT id FROM guilds WHERE discord_id = ?", guild_id)
        .fetch_one(&ctx.data().pool)
        .await?)
}

pub fn force_guild_id(guild: Option<GuildId>) -> i64 {
    match guild {
        Some(guild) => guild.get() as i64,
        None => -1,
    }
}

pub fn matches_to_vecint(captures: &Captures) -> Result<Vec<Option<i32>>, Error> {
    let mut int_matches = Vec::new();
    for capture in captures.iter().skip(1) {
        match capture {
            Some(c) => {
                let Ok(parsed_amount) = c.as_str().parse::<i32>() else {
                    return Err("Um, I-I'm having trouble parsing the arguments... C-could you check them and try again?".into());
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
            return Err("Um, I-I'm having trouble parsing the arguments... C-could you check them and try again?".into());
        };
        let Some(multiplied_amount) = c.checked_mul(*rhs) else {
            return Err("Um, I-I'm having trouble parsing the arguments... C-could you check them and try again?".into());
        };
        if amount.checked_add(multiplied_amount).is_none() {
            return Err("Um, I-I'm having trouble parsing the arguments... C-could you check them and try again?".into());
        };
        amount += multiplied_amount;
    }
    Ok(amount)
}

pub fn parse_utc_offset(data: &Arc<Data>, offset: &str) -> Result<i32, Error> {
    let regex = &data.regex_cache.utc_offset;
    let Some(captures) = &regex.captures(offset) else { return Err("Uh, it looks like the offset is invalid... C-could you check it and try again?".into()) };
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

pub async fn get_author_utc_offset(ctx: &Context<'_>) -> Result<i64, Error> {
    ensure_user_in_db(ctx.data(), ctx.author().id).await?;
    let author_id = ctx.author().id.get() as i64;
    Ok(query_scalar!(r"SELECT utc_offset FROM users WHERE (discord_id) = (?)", author_id)
        .fetch_one(&ctx.data().pool)
        .await?)
}
