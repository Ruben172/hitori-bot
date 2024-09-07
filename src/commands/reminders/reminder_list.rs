use crate::commands::util::force_guild_id;
use crate::util::{paginate, url_guild_id};
use crate::{Context, Error};
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
    let guild_id = force_guild_id(ctx.guild_id());
    let reminders = query!(
        r"SELECT r.id, message, timestamp, c.discord_id AS channel_id, g.discord_id AS guild_id, message_id
        FROM reminders r
        JOIN reminder_user ru ON r.id = ru.reminder_id JOIN users u on ru.user_id = u.id
        JOIN reminder_channel rc ON r.id = rc.reminder_id JOIN channels c on rc.channel_id = c.id
        JOIN reminder_guild rg on r.id = rg.reminder_id JOIN guilds g on rg.guild_id = g.id
        WHERE u.discord_id = ? AND (g.discord_id = ? OR ? = -1) AND active = 1 ORDER BY timestamp ASC",
        author_id, guild_id, guild_id
    )
    .fetch_all(&ctx.data().pool)
    .await?;
    if reminders.is_empty() {
        return Err("You have no active reminders.".into());
    }
    let mut reminder_pages = Vec::<Vec<String>>::new();
    for (i, r) in reminders.iter().enumerate() {
        let reminder_string = format!("ID: {0} · <t:{1}:f> · `{2}` ([Context](https://hitori.discord.com/channels/{3}/{4}/{5}))", r.id, r.timestamp, r.message, url_guild_id(r.guild_id), r.channel_id, r.message_id);
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
