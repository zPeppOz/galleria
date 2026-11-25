# Handoff — fase 2, blocco B2 (design system e guscio)

## Session Metadata

- Data: 2026-11-25, task kanban `t_fa55d26a`, esecutore Genny.
- Worktree: /home/g7aro/dev/galleria/.worktrees/t_fa55d26a, ramo `galleria/t_fa55d26a-galleria-fase-2-blocco-b2-design-system`.
- Partito da `main` `1b3d588` più il piano `0c1584e` (ramo `t_24690770`, ff). Nessun push.
- Piano: `docs/superpowers/plans/2026-11-25-fase2-b2-design-system.md`. Esito vivo: `docs/08-esito-fase2.md`, sezione «B2».
- Q22–Q26 approvate da Aitano il 2026-11-25 su delega di Peppe, tutte come raccomandate (segnate nella spec §18, commit del T1).

## Work Completed

| Task | Contenuto |
| --- | --- |
| T1 | Q12 verificata: **Base UI 1.9.0 regge** (nessuna condizione di stop scattata, niente Radix). `design/{Dialog,Menu,icons}.tsx`, `app/{AppMenu,AboutDialog}.tsx`, controllo licenze JS, `scripts/gui.sh a11y` con Orca + AT-SPI, `ui/e2e/{support,overlays.a11y}.ts` |

## Scelte da sapere (T1)

- Il canale di verifica funziona: Orca 46.1 nell'immagine legge la webview, le righe `SPEECH OUTPUT` e l'albero AT-SPI stanno in
  `target/gui-a11y/` (ignorato da git); la trascrizione completa è in `docs/08-esito-fase2.md`.
- Orca apre il file di debug con buffer a blocchi: `lab/tests/orca-line-buffered.py` lo rende a righe (senza, le prove non
  vedono la voce se non a fine esecuzione).
- WebKitGTK esporta `<button aria-haspopup="menu">` come `combo box` senza `expanded`, anche per un bottone nudo (prova di
  controllo nel file); il piano si aspettava `push button` + `expanded`. Le righe M1/M2 leggono `aria-expanded` dal DOM.
- `Menu.RadioItem` non chiude il menu al clic di predefinito: `closeOnClick` lo chiude (M5).
- `lab/tests/gui-lib.sh` è il pezzo condiviso (cartella temporanea, `daemon-start`/`daemon-stop`, driver) di `gui-smoke.sh` e
  `gui-a11y.sh`. Il fumo di B1 è rimasto identico (5 prove su 5).
- `expectNoAccessibilityViolations(container, { landmarks: false })` spegne la regola `region` per un componente in un portale.
- `AppMenu` tiene la lingua scelta nello stato di `App` (il T3 la sposta in `prefs`).
- Strumenti di sessione: heredoc e `python3 -c` sono bloccati in modalità non presidiata: i file si scrivono con gli strumenti
  di file; per Docker `scripts/gui.sh run` fa da tramite (con `GUI_CONTAINER_NAME=t_fa55d26a-…`).
