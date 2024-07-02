use crate::commands::reminders::util::{serialize_reminder, Reminder};
use crate::commands::util::{message_id_from_ctx, referenced_from_ctx};
use crate::{Context, Error, BOT_COLOR};
use chrono::{Utc};
use poise::serenity_prelude::{CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter};
use poise::{CreateReply};
use regex::Captures;
use sqlx::{query};

fn relative_matches_to_seconds(captures: Captures) -> Result<i32, &str> {
    let second_conversions: [i32; 7] = [31557600, 2629800, 604800, 86400, 3600, 60, 1]; // year, month, week, day, hour, minute, second
    let mut seconds: i32 = 0;
    for (i, c) in captures.iter().skip(1).enumerate() {
        let Some(c) = c else {
            continue;
        };
        let Ok(parsed_length) = c.as_str().parse::<i32>() else {
            return Err("Duration too long!");
        };
        let Some(parsed_seconds) = parsed_length.checked_mul(second_conversions[i]) else {
            return Err("Duration too long!");
        };
        if seconds.checked_add(parsed_seconds).is_none() {
            return Err("Duration too long!");
        };
        seconds += parsed_seconds;
    }
    if seconds > 34560000 {
        return Err("Duration too long!");
    };
    Ok(seconds)
}

/// Create a reminder
///
/// h!remindme <timestamp> <message>
#[poise::command(slash_command, prefix_command, aliases("rm", "rember", "reminder", "remind", "dothething","BOCCHIDONTYOUDAREFORGET"))]
pub async fn remindme(
    ctx: Context<'_>,
    #[description = "When you want to be reminded"] timestamp: String,
    #[description = "What you would like to be reminded of"] #[rest] mut message: Option<String>,
) -> Result<(), Error> {
    let author_id = ctx.author().id.get() as i64;
    let reminder_count = query!(r"SELECT COUNT(*) AS count FROM reminders WHERE user_ids LIKE '%'||?||'%' AND active = 1", author_id).fetch_one(&ctx.data().pool).await?.count;
    if reminder_count > 25 {
        ctx.send(CreateReply::default().content("You have too many active reminders").ephemeral(true)).await?;
        return Ok(())
    }
    let regex_cache = &ctx.data().regex_cache;
    let relative_time = &regex_cache.relative_time;
    if let Some(captures) = relative_time.captures(&timestamp) {
        let seconds = relative_matches_to_seconds(captures)?;
        let timestamp = Utc::now().timestamp() + seconds as i64;
        if let Some(reference) = referenced_from_ctx(ctx) {
            if message.is_none() && !reference.content.is_empty() {
                message = Some(reference.content);
            }
        }
        let mut reminder = Reminder {
            id: None,
            timestamp,
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
    } else {
        ctx.send(CreateReply::default().content("Invalid timestamp.").ephemeral(true)).await?;
    }
    Ok(())
}
