//! Monotonic deadlines and public opaque session representation.
use super::Session;
use serde::Serialize;
use std::sync::atomic::Ordering;
#[derive(Serialize)]
pub(in crate::execution) struct View {
    session_id: String,
    host_id: String,
    created_at: u64,
    idle_expires_at: u64,
    absolute_expires_at: u64,
}

impl Session {
    pub fn active(&self) -> bool {
        let elapsed = self.created.elapsed().as_secs();
        !self.closed.load(Ordering::SeqCst)
            && elapsed < 3600
            && elapsed.saturating_sub(self.touched.load(Ordering::SeqCst)) < 300
    }

    pub fn touch(&self) {
        self.touched
            .store(self.created.elapsed().as_secs(), Ordering::SeqCst);
    }

    pub fn view(&self) -> View {
        View {
            session_id: self.id.clone(),
            host_id: self.profile.host_id.clone(),
            created_at: self.created_at,
            idle_expires_at: self.created_at + self.touched.load(Ordering::SeqCst) + 300,
            absolute_expires_at: self.created_at + 3600,
        }
    }
}
