use crate::commands::reminders::util::{
    cache_reminder, check_author_reminder_count,
    parse_timestamp,
};
use crate::commands::util::{get_author_utc_offset, get_internal_channel_id, get_internal_user_id, message_id_from_ctx, referenced_from_ctx};
use crate::{BOT_COLOR, Context, Error};
use chrono::Utc;
use poise::serenity_prelude::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter};
use poise::CreateReply;
use sqlx::query;

const MAX_REMINDER_SECONDS: i64 = 34560000;

/// Create a reminder
///
/// h!remindme <timestamp> <message>
#[poise::command(
    slash_command,
    prefix_command,
    aliases("rm", "rember", "reminder", "remind", "dothething"),
    check="check_author_reminder_count"
)]
pub async fn remindme(
    ctx: Context<'_>, #[description = "When you want to be reminded"] timestamp: String,
    #[description = "What you would like to be reminded of"]
    #[rest]
    mut message: Option<String>,
) -> Result<(), Error> {
    let offset = get_author_utc_offset(&ctx).await?;
    let unix_timestamp = parse_timestamp(ctx.data(), &timestamp, offset)?;
    if unix_timestamp > Utc::now().timestamp() + MAX_REMINDER_SECONDS {
        return Err("Reminder duration too long.".into());
    };
    if unix_timestamp < Utc::now().timestamp() {
        return Err("Reminder must be in the future!".into());
    }
    if let Some(reference) = referenced_from_ctx(ctx) {
        if message.is_none() && !reference.content.is_empty() {
            message = Some(reference.content);
        }
    }
    let message = message.unwrap_or("something".into());
    let message_id = message_id_from_ctx(ctx).get() as i64;
    let created_at = ctx.created_at().unix_timestamp();

    let i_user_id = get_internal_user_id(ctx.data(), ctx.author().id).await?;
    let i_channel_id = get_internal_channel_id(ctx.data(), ctx.channel_id()).await?;

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
        .title(format!("Reminder #{reminder_id} created."))
        .description(format!(
            "I will remind you <t:{unix_timestamp}:R> on <t:{unix_timestamp}:F> about {message}"
        ))
        .footer(CreateEmbedFooter::new(format!(
            "Tip: use \"{0}follow {1}\" to also get notified for this reminder!",
            ctx.prefix(),
            reminder_id
        )));
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}
