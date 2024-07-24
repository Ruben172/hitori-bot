use crate::commands::util::ensure_user_in_db;
use crate::commands::util::parse_utc_offset;
use crate::{Context, Error, BOT_COLOR};
use poise::serenity_prelude::{CreateEmbed, CreateEmbedAuthor};
use poise::CreateReply;
use sqlx::query;

/// Set your UTC offset
///
/// Example: h!setoffset +02:00
#[poise::command(
    slash_command,
    prefix_command,
    rename = "setoffset",
    aliases("setutcoffset", "setutc", "utcoffset"),
    discard_spare_arguments
)]
pub async fn set_utc_offset(
    ctx: Context<'_>, #[description = "UTC offset"] offset: String,
) -> Result<(), Error> {
    let offset_minutes = parse_utc_offset(ctx.data(), &offset)?;
    ensure_user_in_db(ctx.data(), ctx.author().id).await?;
    let author_id = ctx.author().id.get() as i64;
    query!("UPDATE users SET utc_offset = ? WHERE discord_id = ?", offset_minutes, author_id,)
        .execute(&ctx.data().pool)
        .await?;

    let embed = CreateEmbed::new()
        .author(CreateEmbedAuthor::from(ctx.author().clone()))
        .color(BOT_COLOR)
        .title("UTC offset set!".to_string())
        .description("TBA".to_string());
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}
