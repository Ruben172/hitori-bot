use crate::commands::reminders::util::{cache_reminder, get_internal_user_id, get_next_reminder_ts, reminder_exists_and_active, user_ids_from_reminder_id};
use crate::util::send_ephemeral_text;
use crate::{Context, Error, BOT_COLOR};
use poise::serenity_prelude::CreateEmbed;
use poise::CreateReply;
use sqlx::query;

/// Unfollow or remove a reminder
///
/// h!unfollow <reminder ID>
#[poise::command(
    slash_command,
    prefix_command,
    aliases("reminderremove", "removerm", "forgor"),
    discard_spare_arguments
)]
pub async fn unfollow(
    ctx: Context<'_>, #[description = "The reminder to stop tracking"] reminder_id: u32,
) -> Result<(), Error> {
    let reminder_id = reminder_id as i64;
    if !reminder_exists_and_active(ctx.data(), reminder_id).await {
        send_ephemeral_text(ctx, "Reminder does not exist or has already expired.").await?;
        return Ok(());
    }
    let user_ids = user_ids_from_reminder_id(ctx.data(), reminder_id).await?;

    let user_id = ctx.author().id;
    if !user_ids.contains(&user_id) {
        send_ephemeral_text(ctx, format!("You are not following this reminder. Use `{}follow {}` to follow this reminder!", ctx.prefix(), reminder_id).as_str()).await?;
        return Ok(());        
    }

    let title: String;
    let ephemeral: bool;
    let i_user_id = get_internal_user_id(ctx.data(), user_id).await?;
    query!(
        "DELETE FROM reminder_user WHERE reminder_id = ? AND user_id = ?",
        reminder_id,
        i_user_id
    )
    .execute(&ctx.data().pool)
    .await?;
    if user_ids.len() > 1 {
        title = format!("You will no longer be notified for reminder #{reminder_id}");
        ephemeral = true;
    } else {
        query!("UPDATE reminders SET active = 0 WHERE id = ?", reminder_id)
            .execute(&ctx.data().pool)
            .await?;
        {
            let mut next_reminder = ctx.data().next_reminder.lock().unwrap();
            *next_reminder = None; // Clear the next reminder, as it is unknown whether this is the reminder being removed or not
        }
        if let Some(reminder) = get_next_reminder_ts(&ctx.data().pool).await {
            cache_reminder(ctx.data(), reminder); // Populate the cached reminder again
        }
        title = format!("Reminder #{reminder_id} has been removed.");
        ephemeral = false;
    }

    let embed = CreateEmbed::new().title(title).color(BOT_COLOR);
    ctx.send(CreateReply::default().embed(embed).ephemeral(ephemeral)).await?;

    Ok(())
}
