use crate::{
    Cmd, CommandRunner, NetError,
    journal::{self, Journal},
    plan::{fixed_cleanup, plan, validate},
};
use std::{
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};
pub struct NetManager<R: CommandRunner> {
    runner: R,
    journal: PathBuf,
    transaction: Mutex<()>,
}
impl<R: CommandRunner> NetManager<R> {
    pub fn new(runner: R, journal: PathBuf) -> Self {
        Self {
            runner,
            journal,
            transaction: Mutex::new(()),
        }
    }
    fn lock(&self) -> Result<MutexGuard<'_, ()>, NetError> {
        self.transaction
            .lock()
            .map_err(|_| NetError::InvalidConfig("network transaction poisoned"))
    }
    pub fn apply(&self, cfg: &galleria_backend::net::NetConfig) -> Result<(), NetError> {
        let _guard = self.lock()?;
        validate(cfg)?;
        if self.journal.try_exists()? {
            return Err(NetError::InvalidConfig(
                "network requires teardown before apply",
            ));
        }
        let steps = plan(cfg);
        let journal = Journal {
            undo: steps.iter().filter_map(|s| s.undo.clone()).collect(),
            interfaces: vec![cfg.interface.clone()],
        };
        journal.write(&self.journal)?;
        let mut completed = Vec::new();
        for step in steps {
            // Failure/timeout does not prove the command made no changes.
            if let Some(undo) = step.undo {
                completed.push(undo);
            }
            if let Err(error) = self.runner.run(&step.apply) {
                if self.undo(&completed).is_ok() {
                    journal::remove(&self.journal)?;
                }
                return Err(if step.apply.program == crate::Program::Resolvectl {
                    NetError::DnsUnavailable(error.to_string())
                } else {
                    error
                });
            }
        }
        Ok(())
    }
    fn undo(&self, commands: &[Cmd]) -> Result<(), NetError> {
        let mut first = None;
        for cmd in commands.iter().rev() {
            if let Err(error) = self.runner.run(cmd)
                && !missing(cmd, &error)
                && first.is_none()
            {
                first = Some(error);
            }
        }
        match first {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
    pub fn teardown(&self) -> Result<(), NetError> {
        let _guard = self.lock()?;
        if let Some(journal) = Journal::read(&self.journal)? {
            self.undo(&journal.undo)?;
            journal::remove(&self.journal)?;
        }
        Ok(())
    }
    pub fn recover(&self) -> Result<(), NetError> {
        let _guard = self.lock()?;
        let journal = Journal::read(&self.journal)?;
        let mut result = journal
            .as_ref()
            .map(|j| self.undo(&j.undo))
            .unwrap_or(Ok(()));
        for cmd in fixed_cleanup() {
            if let Err(error) = self.runner.run(&cmd)
                && !missing(&cmd, &error)
                && result.is_ok()
            {
                result = Err(error);
            }
        }
        result?;
        journal::remove(&self.journal)
    }
}
fn missing(cmd: &Cmd, error: &NetError) -> bool {
    if cmd.program == crate::Program::Resolvectl {
        if matches!(error, NetError::DnsUnavailable(_)) {
            return true;
        }
        if let NetError::Command { stderr, .. } = error
            && [
                "Failed to connect to bus",
                "Connection refused",
                "org.freedesktop.resolve1 was not provided",
                "Unit dbus-org.freedesktop.resolve1.service not found",
            ]
            .iter()
            .any(|text| stderr.contains(text))
        {
            return true;
        }
    }
    if cmd.program == crate::Program::Ip
        && cmd.args.first().is_some_and(|a| a == "-6")
        && matches!(error, NetError::Command { stderr, .. } if stderr.contains("Address family not supported by protocol"))
    {
        return true;
    }
    matches!(error,NetError::Command{stderr,..} if stderr.contains("No such device") || stderr.contains("No such file or directory") || stderr.contains("No such process") || stderr.contains("Cannot find device") || stderr.contains("not known"))
}
