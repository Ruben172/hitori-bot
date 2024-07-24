pub mod reminders;
mod util;
mod utility;

pub fn commands() -> Vec<crate::Command> {
    reminders::commands()
        .into_iter()
        .chain(utility::commands())
        // .chain(utility::commands())
        .collect()
}
