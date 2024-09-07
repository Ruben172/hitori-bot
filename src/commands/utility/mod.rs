mod set_utc_offset;
mod set_fallback_channel;

pub fn commands() -> [crate::Command; 2] {
    [
        set_utc_offset::set_utc_offset(),
        set_fallback_channel::set_fallback_channel()
    ]
}
