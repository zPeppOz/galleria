use crate::{
    Cmd, CommandRunner, NetError,
    journal::{self, Journal},
    plan::{fixed_cleanup, plan, validate},
};
use std::{
    collections::VecDeque,
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
        let mut pending: VecDeque<_> = commands.iter().rev().cloned().collect();
        while let Some(cmd) = pending.pop_front() {
            if let Err(error) = self.runner.run(&cmd)
                && !missing(&error)
            {
                if cmd.program == crate::Program::Resolvectl
                    && let [action, interface] = cmd.args.as_slice()
                    && action == "revert"
                {
                    // Removing this link also discards resolved's per-link state.
                    pending.push_back(Cmd {
                        program: crate::Program::Ip,
                        args: vec!["link".into(), "del".into(), interface.clone()],
                    });
                } else if first.is_none() {
                    first = Some(error);
                }
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
                && !missing(&error)
                && result.is_ok()
            {
                result = Err(error);
            }
        }
        result?;
        journal::remove(&self.journal)
    }
}
fn missing(error: &NetError) -> bool {
    matches!(error,NetError::Command{stderr,..} if stderr.contains("No such device") || stderr.contains("No such file or directory") || stderr.contains("No such process") || stderr.contains("Cannot find device") || stderr.contains("not known"))
}
