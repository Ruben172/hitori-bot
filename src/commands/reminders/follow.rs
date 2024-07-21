use crate::commands::reminders::util::{
    check_author_reminder_count, get_internal_user_id, reminder_exists_and_active,
    user_ids_from_reminder_id,
};
use crate::util::send_ephemeral_text;
use crate::{Context, Error, BOT_COLOR};
use poise::serenity_prelude::CreateEmbed;
use poise::CreateReply;
use sqlx::query;

/// Follow someone else's reminder
///
/// h!follow <reminder ID>
#[poise::command(
    slash_command,
    prefix_command,
    discard_spare_arguments,
    check="check_author_reminder_count"
)]
pub async fn follow(
    ctx: Context<'_>, #[description = "The reminder to track"] reminder_id: u32,
) -> Result<(), Error> {
    let reminder_id = reminder_id as i64;
    if !reminder_exists_and_active(ctx.data(), reminder_id).await {
        send_ephemeral_text(ctx, "Reminder does not exist or has already expired.").await?;
        return Ok(());
    }
    let user_ids = user_ids_from_reminder_id(ctx.data(), reminder_id).await?;
    let user_id = ctx.author().id;
    if user_ids.contains(&user_id) {
        send_ephemeral_text(ctx, "You are already following this reminder.").await?;
        return Ok(());
    }

    let i_user_id = get_internal_user_id(ctx.data(), user_id).await?;
    query!(
        "INSERT INTO reminder_user (reminder_id, user_id) VALUES (?, ?)",
        reminder_id,
        i_user_id
    )
    .execute(&ctx.data().pool)
    .await?;

    let embed = CreateEmbed::new()
        .title(format!("You will now be notified for reminder #{reminder_id}!"))
        .color(BOT_COLOR);

    ctx.send(CreateReply::default().embed(embed).ephemeral(true)).await?;
    Ok(())
}
