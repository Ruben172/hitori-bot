use crate::{Context, Error, BOT_COLOR};
use poise::serenity_prelude::{
    ComponentInteractionCollector, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedAuthor,
    CreateEmbedFooter, CreateInteractionResponse, CreateInteractionResponseMessage, EmojiId,
    ReactionType,
};
use poise::CreateReply;

fn create_page_embed(
    ctx: Context<'_>, pages: Vec<Vec<String>>, title: String, page: usize,
) -> CreateEmbed {
    CreateEmbed::default()
        .color(BOT_COLOR)
        .author(CreateEmbedAuthor::from(ctx.author().clone()))
        .title(title)
        .description(pages[page].join("\n"))
        .footer(CreateEmbedFooter::new(format!(
            "Page {}/{} - Showing entries {}-{} out of {}.",
            page + 1,
            pages.len(),
            page * pages[0].len() + 1,
            page * pages[0].len() + pages[page].len(),
            pages[0].len() * (pages.len() - 1) + pages[pages.len() - 1].len()
        )))
}

pub async fn paginate(
    ctx: Context<'_>, pages: Vec<Vec<String>>, title: String, mut page: usize,
) -> Result<(), Error> {
    // Define some unique identifiers for the navigation buttons
    let ctx_id = ctx.id();
    if page >= pages.len() {
        page = 0;
    }
    let prev_button_id = format!("{}prev", ctx_id);
    let next_button_id = format!("{}next", ctx_id);

    // Send the embed with the first page as content
    let mut reply = {
        CreateReply::default().embed(create_page_embed(
            ctx,
            pages.clone(),
            title.clone(),
            page.clone(),
        ))
    };

    if pages.len() > 1 {
        let components = CreateActionRow::Buttons(vec![
            CreateButton::new(&prev_button_id).emoji(ReactionType::Custom {
                animated: false,
                id: EmojiId::new(1257787809633275954),
                name: Some("bwaaa_left".into()),
            }),
            CreateButton::new(&next_button_id).emoji(ReactionType::Custom {
                animated: false,
                id: EmojiId::new(1257787824283844772),
                name: Some("bwaaa_right".into()),
            }),
        ]);
        reply = reply.components(vec![components])
    }

    ctx.send(reply).await?;

    if pages.len() == 1 {
        return Ok(());
    }

    // Loop through incoming interactions with the navigation buttons
    while let Some(press) = ComponentInteractionCollector::new(ctx)
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        // Timeout when no navigation button has been pressed for 24 hours
        .timeout(std::time::Duration::from_secs(120))
        .await
    {
        // Depending on which button was pressed, go to next or previous page
        if press.data.custom_id == next_button_id {
            page += 1;
            if page >= pages.len() {
                page = 0;
            }
        } else {
            page = page.checked_sub(1).unwrap_or(pages.len() - 1);
        }

        // Update the message with the new page contents
        press
            .create_response(
                ctx.serenity_context(),
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new().embed(create_page_embed(
                        ctx,
                        pages.clone(),
                        title.clone(),
                        page.clone(),
                    )),
                ),
            )
            .await?;
    }

    Ok(())
}

pub async fn send_ephemeral_text(ctx: Context<'_>, content: &str) -> Result<(), Error> {
    ctx.send(CreateReply::default().content(content).ephemeral(true)).await?;
    Ok(())
}
