mod follow;
mod reminder_list;
mod remindme;
pub mod util;

pub fn commands() -> [crate::Command; 3] {
    [remindme::remindme(), reminder_list::reminder_list(), follow::follow()]
}
