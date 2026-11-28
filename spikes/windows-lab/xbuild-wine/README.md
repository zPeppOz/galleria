# Le prove di Windows (gnu) sotto wine

- Immagine: `docker build -t galleria-xbuild-wine spikes/windows-lab/xbuild-wine` (poi `run.sh` la usa; il registro di Cargo sta in `~/.cache/galleria-xcargo`).
- Comando misurato: `spikes/windows-lab/xbuild-wine/run.sh test --workspace --exclude galleria-e2e --no-fail-fast` → 605 prove, 603 passate, 2 fallite: `a_saved_profile_is_not_readable_by_other_users` e `a_link_is_not_imported`, che wine non sa provare. Il nome del container è `XWINE_CONTAINER_NAME` (default `galleria-xwine`); gli argomenti vanno a `cargo`.
- `ui/dist` deve esistere (`pnpm --dir ui build`, o copiarla): la finestra la incorpora a compile time, e un worktree nuovo non l'ha.
