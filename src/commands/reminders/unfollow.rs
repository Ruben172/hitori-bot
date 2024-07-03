use crate::commands::reminders::util::{
    cache_reminder, get_next_reminder, serialize_user_ids, user_ids_from_reminder_id,
};
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
    let Ok(mut user_ids) = user_ids_from_reminder_id(ctx, reminder_id).await else {
        return Ok(())
    };

    match user_ids.iter().position(|&x| x == ctx.author().id) {
        Some(item) => user_ids.remove(item),
        None => {
            send_ephemeral_text(ctx, format!("You are not following this reminder. Use `{}follow {}` to follow this reminder!", ctx.prefix(), reminder_id).as_str()).await?;
            return Ok(());
        }
    };

    let serialized_user_ids = serialize_user_ids(&user_ids);
    let title: String;
    let ephemeral: bool;
    if user_ids.len() > 0 {
        query!("UPDATE reminders SET user_ids = ? WHERE id = ?", serialized_user_ids, reminder_id)
            .execute(&ctx.data().pool)
            .await?;
        let mut next_reminder = ctx.data().next_reminder.lock().unwrap();
        if let Some(stored_reminder) = &mut *next_reminder {
            if stored_reminder.id == Some(reminder_id) {
                stored_reminder.user_ids = user_ids;
            }
        };
        title = format!("You will no longer be notified for reminder #{}", reminder_id);
        ephemeral = true;
    } else {
        query!("UPDATE reminders SET active = 0 WHERE id = ?", reminder_id)
            .execute(&ctx.data().pool)
            .await?;
        {
            let mut next_reminder = ctx.data().next_reminder.lock().unwrap();
            if let Some(stored_reminder) = &mut *next_reminder {
                if stored_reminder.id == Some(reminder_id) {
                    *next_reminder = None;
                }
            };
        }
        if let Some(mut reminder) = get_next_reminder(&ctx.data().pool).await {
            cache_reminder(&ctx.data(), &mut reminder);
        }
        title = format!("Reminder #{} has been removed.", reminder_id);
        ephemeral = false;
    }

    let embed = CreateEmbed::new().title(title).color(BOT_COLOR);
    ctx.send(CreateReply::default().embed(embed).ephemeral(ephemeral)).await?;

    Ok(())
}
