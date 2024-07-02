use crate::Context;
use poise::serenity_prelude::{Message, MessageId};

pub fn message_id_from_ctx(ctx: Context<'_>) -> MessageId {
    match ctx {
        Context::Application(actx) => actx.interaction.id.get().into(),
        Context::Prefix(pctx) => pctx.msg.id,
    }
}

pub fn referenced_from_ctx(ctx: Context<'_>) -> Option<Message> {
    match ctx {
        Context::Application(_actx) => None,
        Context::Prefix(pctx) => pctx.msg.referenced_message.as_ref().map(|m| *m.clone()),
    }
}
