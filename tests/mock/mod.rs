//! A controllable HTTPS issuer exercises failures a genuine realm cannot inject.
mod handler;
pub mod scenarios;
mod server;
use anyhow::{Result, ensure};
pub use server::start;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

pub struct State {
    pub issuer: String,
    pub counts: Mutex<HashMap<String, usize>>,
}
pub struct Mock {
    pub state: Arc<State>,
    task: tokio::task::JoinHandle<()>,
}
impl Mock {
    pub fn count(&self, token: &str) -> Result<usize> {
        let counts = self
            .state
            .counts
            .lock()
            .map_err(|_| anyhow::anyhow!("mock state poisoned"))?;
        Ok(*counts.get(token).unwrap_or(&0))
    }
    pub fn assert_no_redirects(&self) -> Result<()> {
        ensure!(self.count("redirect-followed")? == 0);
        Ok(())
    }
}
impl Drop for Mock {
    fn drop(&mut self) {
        self.task.abort();
    }
}
