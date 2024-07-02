pub mod reminders;
mod util;

pub fn commands() -> Vec<crate::Command> {
    reminders::commands()
        .into_iter()
        // .chain(reminders::commands())
        .collect()
}
