use crate::commands::reminders::util::{
    check_author_reminder_count, serialize_user_ids, user_ids_from_reminder_id,
};
use crate::util::send_ephemeral_text;
use crate::{Context, Error, BOT_COLOR};
use poise::serenity_prelude::CreateEmbed;
use poise::CreateReply;
use sqlx::query;

/// Follow someone else's reminder
///
/// h!follow <reminder ID>
#[poise::command(slash_command, prefix_command, discard_spare_arguments)]
pub async fn follow(
    ctx: Context<'_>, #[description = "The reminder to track"] reminder_id: u32,
) -> Result<(), Error> {
    if check_author_reminder_count(ctx).await.is_err() {
        return Ok(());
    }
    let Ok(mut user_ids) = user_ids_from_reminder_id(ctx, reminder_id).await else { return Ok(()) };
    if user_ids.contains(&ctx.author().id) {
        send_ephemeral_text(ctx, "You are already following this reminder.").await?;
        return Ok(());
    }

    user_ids.push(ctx.author().id);
    let serialized_user_ids = serialize_user_ids(&user_ids);
    query!("UPDATE reminders SET user_ids = ? WHERE ID = ?", serialized_user_ids, reminder_id)
        .execute(&ctx.data().pool)
        .await?;
    {
        let mut next_reminder = ctx.data().next_reminder.lock().unwrap();
        if let Some(stored_reminder) = &mut *next_reminder {
            if stored_reminder.id == Some(reminder_id) {
                stored_reminder.user_ids = user_ids;
            }
        };
    }
    let embed = CreateEmbed::new()
        .title(format!("You will now be notified for reminder #{reminder_id}!"))
        .color(BOT_COLOR);

    ctx.send(CreateReply::default().embed(embed).ephemeral(true)).await?;
    Ok(())
}
