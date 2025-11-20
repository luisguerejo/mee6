#[derive(Debug, PartialEq)]
pub enum Status {
    Playing,
    Idle,
    Paused,
    Disconnected,
}

impl Status {
    pub fn should_enqueue(current_status: Status) -> bool {
        return current_status != Status::Disconnected || current_status != Status::Idle
    }
}
