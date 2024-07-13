use crate::commands::reminders::util::{
    check_author_reminder_count, parse_timestamp, serialize_reminder, Reminder,
};
use crate::commands::util::{message_id_from_ctx, referenced_from_ctx};
use crate::util::send_ephemeral_text;
use crate::{Context, Error, BOT_COLOR};
use chrono::Utc;
use poise::serenity_prelude::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter};
use poise::CreateReply;

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
    let mut reminder = Reminder {
        id: None,
        timestamp: unix_timestamp,
        created_at: ctx.created_at().unix_timestamp(),
        user_ids: vec![ctx.author().id],
        channel_id: ctx.channel_id(),
        message_id: message_id_from_ctx(ctx),
        message: message.unwrap_or("something".into()),
    };
    serialize_reminder(ctx, &mut reminder).await?;
    let embed = CreateEmbed::new()
        .author(CreateEmbedAuthor::from(ctx.author().clone()))
        .color(BOT_COLOR)
        .title(format!("Reminder #{0} created.", reminder.id.unwrap()))
        .description(format!(
            "I will remind you <t:{0}:R> on <t:{0}:F> about {1}",
            reminder.timestamp, reminder.message
        ))
        .footer(CreateEmbedFooter::new(format!(
            "Tip: use \"{0}follow {1}\" to also get notified for this reminder!",
            ctx.prefix(),
            reminder.id.unwrap()
        )));
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}
