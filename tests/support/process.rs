//! Own the child process so every error path kills and reaps the test server.
use anyhow::{Context, Result, ensure};
use std::{
    collections::BTreeMap,
    fs::File,
    net::TcpStream,
    process::{Child, Command, Stdio},
};
use tokio::time::{Duration, sleep};

pub type Environment = BTreeMap<String, String>;
pub struct Postern {
    child: Child,
    logs: File,
}

pub fn command(env: &Environment) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_postern"));
    for (name, _) in
        std::env::vars_os().filter(|(k, _)| k.to_string_lossy().starts_with("POSTERN_"))
    {
        command.env_remove(name);
    }
    command.envs(env);
    command
}

impl Postern {
    pub async fn start(env: &Environment) -> Result<Self> {
        let logs = tempfile::tempfile()?;
        let child = command(env)
            .stdout(Stdio::from(logs.try_clone()?))
            .stderr(Stdio::from(logs.try_clone()?))
            .spawn()?;
        let mut server = Self { child, logs };
        for _ in 0..100 {
            ensure!(
                server.child.try_wait()?.is_none(),
                "Postern exited before readiness"
            );
            if TcpStream::connect_timeout(&"127.0.0.1:8765".parse()?, Duration::from_millis(100))
                .is_ok()
            {
                return Ok(server);
            }
            sleep(Duration::from_millis(50)).await;
        }
        anyhow::bail!("Postern readiness timed out")
    }

    pub fn assert_private_logs(&mut self, secrets: &[&str]) -> Result<()> {
        use std::io::{Read, Seek, SeekFrom};
        self.logs.seek(SeekFrom::Start(0))?;
        let mut output = String::new();
        self.logs
            .read_to_string(&mut output)
            .context("read child diagnostics")?;
        for secret in secrets {
            ensure!(!output.contains(secret), "secret appeared in diagnostics");
        }
        Ok(())
    }
}

impl Drop for Postern {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
