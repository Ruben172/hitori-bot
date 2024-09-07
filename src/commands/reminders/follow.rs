use crate::commands::reminders::util::{
    check_author_reminder_count, guild_from_reminder_id, reminder_exists_and_active,
    user_ids_from_reminder_id,
};
use crate::commands::util::{force_guild_id, get_internal_user_id};
use crate::{Context, Error, BOT_COLOR};
use poise::serenity_prelude::CreateEmbed;
use poise::CreateReply;
use sqlx::{query, query_scalar};

/// Follow someone else's reminder
///
/// h!follow <reminder ID>
#[poise::command(
    slash_command,
    prefix_command,
    discard_spare_arguments,
    guild_only,
    check = "check_author_reminder_count"
)]
pub async fn follow(
    ctx: Context<'_>, #[description = "The reminder to track"] reminder_id: Option<u32>,
) -> Result<(), Error> {
    let reminder_id = match reminder_id {
        Some(reminder_id) => reminder_id as i64,
        None => {
            let Some(guild_id) = ctx.guild_id() else {
                return Err("something really weird happened and the guild-only command returned a guild that's not actually a guild".into())
            };
            let guild_id = guild_id.get() as i64;
            query_scalar!(
                r"SELECT r.id
                FROM reminders r
                JOIN reminder_guild rg ON r.id = rg.reminder_id 
                JOIN guilds g ON rg.guild_id = g.id
                WHERE active = 1 AND g.discord_id = ? ORDER by created_at DESC LIMIT 1", guild_id
            ).fetch_one(&ctx.data().pool).await.map_err(|_| "No active reminders in this guild")?
        }
    };
    if !reminder_exists_and_active(ctx.data(), reminder_id).await {
        return Err("U-um... it looks like the reminder doesn't exist anymore... or it's already expired... S-sorry about that!".into());
    }
    let user_ids = user_ids_from_reminder_id(ctx.data(), reminder_id).await?;
    let user_id = ctx.author().id;
    if user_ids.contains(&user_id) {
        return Err("Oh, um... it seems you're already following this reminder... so, I-I can't add it again. Sorry about that!".into());
    }
    let guild_id = guild_from_reminder_id(ctx.data(), reminder_id).await?;
    if guild_id != force_guild_id(ctx.guild_id()) {
        return Err("Um, it seems this reminder isn't from this guild... S-sorry, but I can't access it here.".into());
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
        .title(format!("Um, y-you'll now be notified for reminder #{reminder_id}! I-I hope that works for you!"))
        .color(BOT_COLOR);

    ctx.send(CreateReply::default().embed(embed).ephemeral(true)).await?;
    Ok(())
}
