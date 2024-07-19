use crate::commands::reminders::util::{
    cache_reminder, check_author_reminder_count, parse_timestamp,
};
use crate::commands::util::{message_id_from_ctx, referenced_from_ctx};
use crate::util::send_ephemeral_text;
use crate::{Context, Error, BOT_COLOR};
use chrono::Utc;
use poise::serenity_prelude::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter};
use poise::CreateReply;
use sqlx::{query, query_scalar};

const MAX_REMINDER_SECONDS: i64 = 34560000;

/// Create a reminder
///
/// h!remindme <timestamp> <message>
#[poise::command(
    slash_command,
    prefix_command,
    aliases("rm", "rember", "reminder", "remind", "dothething", "BOCCHIDONTYOUDAREFORGET")
)]
pub async fn remindme(
    ctx: Context<'_>, #[description = "When you want to be reminded"] timestamp: String,
    #[description = "What you would like to be reminded of"]
    #[rest]
    mut message: Option<String>,
) -> Result<(), Error> {
    if check_author_reminder_count(ctx).await.is_err() {
        return Ok(());
    }
    let Ok(unix_timestamp) = parse_timestamp(ctx.data(), &timestamp) else {
        send_ephemeral_text(ctx, "Invalid timestamp.").await?;
        return Ok(());
    };
    if unix_timestamp > Utc::now().timestamp() + MAX_REMINDER_SECONDS {
        send_ephemeral_text(ctx, "Reminder duration too long.").await?;
        return Ok(());
    };
    if unix_timestamp < Utc::now().timestamp() {
        send_ephemeral_text(ctx, "Reminder must be in the future!").await?;
        return Ok(());
    }
    if let Some(reference) = referenced_from_ctx(ctx) {
        if message.is_none() && !reference.content.is_empty() {
            message = Some(reference.content);
        }
    }
    let message = message.unwrap_or("something".into());
    let author_id = ctx.author().id.get() as i64;
    let channel_id = ctx.channel_id().get() as i64;
    let message_id = message_id_from_ctx(ctx).get() as i64;
    let created_at = ctx.created_at().unix_timestamp();

    query!(r"INSERT OR IGNORE INTO users (discord_id) VALUES (?)", author_id)
        .execute(&ctx.data().pool)
        .await?;
    let i_user_id = query_scalar!(r"SELECT id FROM users WHERE discord_id = ?", author_id)
        .fetch_one(&ctx.data().pool)
        .await?;

    query!(r"INSERT OR IGNORE INTO channels (discord_id) VALUES (?)", channel_id)
        .execute(&ctx.data().pool)
        .await?;
    let i_channel_id = query_scalar!(r"SELECT id FROM channels WHERE discord_id = ?", channel_id)
        .fetch_one(&ctx.data().pool)
        .await?;

    let reminder_id = query!(
        "INSERT INTO reminders (message, timestamp, created_at, message_id) VALUES (?, ?, ?, ?)",
        message,
        unix_timestamp,
        created_at,
        message_id
    )
    .execute(&ctx.data().pool)
    .await?
    .last_insert_rowid();

    query!(
        r"INSERT INTO reminder_user (reminder_id, user_id) VALUES (?, ?)",
        reminder_id,
        i_user_id
    )
    .execute(&ctx.data().pool)
    .await?;
    query!(
        r"INSERT INTO reminder_channel (reminder_id, channel_id) VALUES (?, ?)",
        reminder_id,
        i_channel_id
    )
    .execute(&ctx.data().pool)
    .await?;

    cache_reminder(ctx.data(), unix_timestamp);
    let embed = CreateEmbed::new()
        .author(CreateEmbedAuthor::from(ctx.author().clone()))
        .color(BOT_COLOR)
        .title(format!("Reminder #{0} created.", reminder_id))
        .description(format!(
            "I will remind you <t:{0}:R> on <t:{0}:F> about {1}",
            unix_timestamp, message
        ))
        .footer(CreateEmbedFooter::new(format!(
            "Tip: use \"{0}follow {1}\" to also get notified for this reminder!",
            ctx.prefix(),
            reminder_id
        )));
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}
