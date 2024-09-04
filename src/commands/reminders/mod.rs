use crate::commands::reminders::remindme::{remindme_slash, remindme_text};

mod follow;
mod reminder_list;
mod remindme;
mod unfollow;
pub mod util;

pub fn commands() -> [crate::Command; 4] {
    let remindme = poise::Command {
        slash_action: remindme_slash().slash_action,
        parameters: remindme_slash().parameters,
        ..remindme_text()
    };

    [
        remindme,
        reminder_list::reminder_list(),
        follow::follow(),
        unfollow::unfollow(),
    ]
}
