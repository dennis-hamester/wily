aldrin::generate! {
    "src/schemas/daemon.aldrin",
    "src/schemas/wily.aldrin",
    struct_builders = false,
}

pub use daemon::*;
pub use wily::*;

impl ShareDisabled {
    pub fn any(self) -> bool {
        self.user
    }
}
