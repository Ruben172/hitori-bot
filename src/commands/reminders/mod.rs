mod follow;
mod reminder_list;
mod remindme;
mod unfollow;
pub mod util;

pub fn commands() -> [crate::Command; 4] {
    [remindme::remindme(), reminder_list::reminder_list(), follow::follow(), unfollow::unfollow()]
}
