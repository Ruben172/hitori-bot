use poise::CreateReply;
use crate::commands::util::{ensure_guild_in_db, get_internal_channel_id};
use crate::{BOT_COLOR, Context, Error};
use poise::serenity_prelude::{ChannelId, ChannelType, CreateEmbed, CreateEmbedAuthor};
use sqlx::query;

/// Set the server's fallback channel
///
/// Example: h!setfallback CHANNELID
#[poise::command(
    slash_command,
    prefix_command,
    rename = "setfallback",
    aliases("fallbackchannel", "setfallbackchannel"),
    discard_spare_arguments,
    required_permissions = "MANAGE_CHANNELS", // remove for a slash-command only bot
    default_member_permissions = "MANAGE_CHANNELS", // this currently only hides the command from the command picker, perms are handled by above
    guild_only
)]
pub async fn set_fallback_channel(
    ctx: Context<'_>,
    #[description = "Channel to be used when other options are unavailable"]
    #[channel_types("Text")]
    channel: Option<ChannelId>,
) -> Result<(), Error> {
    let channel = match channel {
        Some(channel) => channel,
        None => ctx.channel_id(),
    };
    let Some(guild_channel) = channel.to_channel(ctx).await?.guild() else {
        return Err("something really weird happened and the guild-only command returned a channel that's not in a guild".into());
    };
    if guild_channel.kind != ChannelType::Text {
        return Err("Ah, you need to specify a text channel... I-I'm afraid only those are supported.".into());
    }
    
    let i_channel_id = get_internal_channel_id(ctx.data(), channel).await?;
    ensure_guild_in_db(ctx, ctx.guild_id()).await?;
    let guild_id = guild_channel.guild_id.get() as i64;
    query!(r"UPDATE guilds SET fallback_channel = ? WHERE discord_id = ?", i_channel_id, guild_id)
        .execute(&ctx.data().pool)
        .await?;
    
    let embed = CreateEmbed::new()
        .author(CreateEmbedAuthor::from(ctx.author().clone()))
        .color(BOT_COLOR)
        .title("Fallback channel updated.")
        .description(format!(
            "Okay, I'll now use <#{}> as the fallback channel. I-I hope that works for you!", channel.get()
        ));
    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}
