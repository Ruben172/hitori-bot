use crate::util::paginate;
use crate::{Context, Error, GUILD_ID};
use sqlx::query;

const PAGE_ITEMS: usize = 8;

/// Shows your list of reminders
///
/// what the fuck
#[poise::command(
    slash_command,
    prefix_command,
    rename = "reminderlist",
    aliases("reminders"),
    discard_spare_arguments
)]
pub async fn reminder_list(
    ctx: Context<'_>, #[description = "The page to start on"] start_page: Option<usize>,
) -> Result<(), Error> {
    let author_id = ctx.author().id.get() as i64;
    let reminders = query!(
        r"SELECT r.id, message, timestamp, c.discord_id AS discord_channel_id, message_id
        FROM reminders r
        JOIN reminder_user ru ON r.id = ru.reminder_id
        JOIN users u on ru.user_id = u.id
        JOIN reminder_channel rc ON r.id = rc.reminder_id
        JOIN channels c on rc.channel_id = c.id
        WHERE u.discord_id = ? AND active = 1 ORDER BY timestamp ASC",
        author_id
    )
    .fetch_all(&ctx.data().pool)
    .await?;
    if reminders.is_empty() {
        ctx.say("You have no active reminders.").await?;
        return Ok(());
    }
    let mut reminder_pages = Vec::<Vec<String>>::new();
    for (i, reminder) in reminders.iter().enumerate() {
        let reminder_string = format!("ID: {0} · <t:{1}:f> · `{2}` ([Context](https://hitori.discord.com/channels/{GUILD_ID}/{3}/{4}))", reminder.id, reminder.timestamp, reminder.message, reminder.discord_channel_id, reminder.message_id);
        if i % PAGE_ITEMS == 0 {
            reminder_pages.push(vec![reminder_string]);
        } else {
            reminder_pages[i / PAGE_ITEMS].push(reminder_string);
        }
    }

    paginate(
        ctx,
        &reminder_pages,
        format!("Active reminders for {}", ctx.author().name),
        start_page.unwrap_or_default(),
    )
    .await
}
