pub mod util;
mod remindme;
mod reminder_list;

pub fn commands() -> [crate::Command; 2] {
    [remindme::remindme(), reminder_list::reminder_list()]
}
