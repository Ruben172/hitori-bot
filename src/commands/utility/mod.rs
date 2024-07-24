mod set_utc_offset;

pub fn commands() -> [crate::Command; 1] {
    [set_utc_offset::set_utc_offset()]
}
