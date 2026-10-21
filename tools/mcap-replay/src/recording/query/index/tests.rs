use super::*;

impl CacheState {
    pub(in crate::recording::query) fn set_budget(&mut self, bytes: usize) -> Result<()> {
        ensure!(
            bytes >= ENTRY_BYTES,
            "index cache must hold at least one {ENTRY_BYTES}-byte entry"
        );
        self.budget = bytes;
        self.make_room(0, 0)
    }
}
