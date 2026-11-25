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
| T2 | `design/{tokens.css,base.css,theme.ts,color.ts}`, prove di contrasto/tema/regole CSS; S8 verde nella finestra vera, S17 falso (via `system-ui`) |
| T3 | `i18n/{en,it}.json` (74 chiavi, con `error.*` per i 19 codici), `i18n/index.tsx`, `prefs.tsx` (lingua e tema in memoria, Q25), `tests/catalogs.rs` + `GUI_CODES` in `error.rs` (G8), gruppo «Tema» nel menu, fumo `language_switches_without_a_restart` |
| T4 | `design/{icons,Button,Field,Switch,Segmented,Tabs}.tsx` + css + prove (axe e tastiera per ciascuno); HTML nativo salvo `Tabs` (Base UI) |

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
- Lingua e tema stanno in `prefs.tsx` (T3), solo in memoria; `AppMenu` li legge da `usePrefs()`. Il menu ha due gruppi radio: `overlays.a11y.ts` aspetta sei radio e due segnati.
- Strumenti di sessione: heredoc e `python3 -c` sono bloccati in modalità non presidiata: i file si scrivono con gli strumenti
  di file; per Docker `scripts/gui.sh run` fa da tramite (con `GUI_CONTAINER_NAME=t_fa55d26a-…`).

## Scelte da sapere (T2)

- WebKitWebDriver: `takeScreenshot` di un elemento restituisce **tutta la pagina**. Per i colori si fotografa la finestra e si legge
  il pixel di un riquadro messo in alto a sinistra (`paint()` in `smoke.e2e.ts`); prima del clic si aspettano due `requestAnimationFrame`.
- `system-ui` non segue il carattere del desktop in WebKitGTK 2.52.6 (provato con `gtk-font-name` serif): tolto da `--font` (F3).
- In questa immagine WebKitGTK dichiara `prefers-color-scheme: light`: con «Sistema» l'app parte chiara.
- `GALLERIA_SMOKE_FILTER='nome'` fa girare una sola prova del fumo (pattern `-t` di vitest); comodo con `scripts/gui.sh run`
  e i binari già costruiti (`target/gui-image/release/galleria-gui`, `target/gui-image/debug/examples/fake-daemon`).
- `styles.css` resta con le misure di B1 (eccezione per nome in `css-rules.test.ts`); sparisce al T6.
- `--tint` è `14%`: le tinte si scrivono `color-mix(in oklch, var(--ok) var(--tint), transparent)`, testo `--ink` sopra.

## Scelte da sapere (T3–T4)

- `errorText(t, code, fallback)`: il codice sconosciuto mostra il messaggio inglese del demone (D22). `StatusView` lo usa.
- `GUI_CODES` in `error.rs` è vuoto: ogni codice `gui_…`/`import_…` va aggiunto lì e nei due cataloghi nello stesso commit
  (`tests/catalogs.rs` lo pretende, anche cercando i letterali nei sorgenti).
- Per aggiungere una variante a `ErrorCode`/`SessionErrorCode`/`State`/`LinkState`: `tests/catalogs.rs` non compila finché non è
  nell'elenco della macro `every_variant!`.
- `Tabs.Panel` di Base UI ha `tabindex="0"`: dalla lista il Tab va al pannello. Il `Tabs.Indicator` non è ancora provato nella
  finestra vera (T7).
- Restano T7 (tastiera, fumo, Orca, esito, roadmap): card successiva. `styles.css` è sparito al T6.

## Scelte da sapere (T5)

- `Toast`: due regioni vive (`status`, `alert`) sempre presenti e vuote, scritte da noi (non il `Toast` di Base UI); l'avviso non
  prende mai il fuoco, resta con il puntatore o il fuoco dentro. `useToast()` dà `(message, {tone})`.
- `InlineAlert` ha `toneLabel` (obbligatoria): è il nome dell'icona (`role="img"`), così il tono non è solo colore. Chiavi nuove
  `alert.*` e `toast.close` nei due cataloghi.
- `ConfirmDialog` è `AlertDialog` di Base UI; i due pulsanti sono `<button class="gx-button …">` perché `initialFocus` vuole un
  riferimento (`Button` non lo inoltra).
- `FirstRun` e `StatusView` hanno `screens/*.css` loro; `App.tsx` avvolge tutto in `ToastProvider` (`toast.close`).

## Scelte da sapere (T6)

- `ui/src/shell/`: `Shell` tiene il `ToastProvider` e non chiama mai il demone (`boundary.test.ts`); `App.tsx` passa solo
  `view`, `info` e `retry` da `useDaemon`. `ui/src/styles.css` non esiste più, `css-rules.test.ts` senza eccezioni.
- Lista: tabindex a rotazione (una sosta, la riga scelta); la scelta segue il fuoco; la prima lettera salta. Il protocollo si dice
  con `ProtocolTag` («WG» nascosto + «WireGuard» in testo nascosto). I titoli dei gruppi vengono dal catalogo.
- `BackendWarnings` è esportato da `screens/StatusView.tsx`; `ServiceArea` lo usa da solo con sessione `disconnected` e lascia
  `StatusView` (che lo contiene) quando c'è una sessione: l'avviso compare una volta sola.
- Per i test del guscio: `getAllByRole("option")` prende anche le `<option>` del `<select>` della scheda Generale: va ristretto a
  `listbox`. Le regioni vive dei toast (`status`, `alert`) ci sono sempre: per cercare un avviso in linea usare `.gx-alert`.
- Il campo di ricerca non c'è (B3). Le stelle (`--accent`) sono `fill`/`stroke`, non `color`: la regola «accento mai testo» le vede.
- Nella finestra vera si vede l'indicatore della scheda (sottolineatura sotto «Generale»); la prova esplicita di nessuna violazione
  della CSP con il guscio resta al T7 (`no_csp_violation_during_the_tour`).
- Merge: il ramo di T5/T6 non era in avanzamento rapido sul ramo di T1–T4 (main aveva i commit delle VM): fatto un merge
  `bbb54e7` invece del `--ff-only`.

## Scelte da sapere (T7)

- `keyboard.test.tsx` (9 prove, solo `user.tab()`/`user.keyboard()`), cinque prove nuove nel fumo (`the shell in the real window`) e
  `ui/e2e/shell.a11y.ts` (11 prove con Orca dopo la correzione di `t_43cd01da`; 8 al T7). Il fumo ora legge i token a livello di modulo (`themes`), non dentro un `describe`.
- **Difetto corretto**: chiudere un avviso con la tastiera lasciava il fuoco sul `body`; `Toast` ora lo restituisce a chi lo aveva.
- **Due buchi aperti** (G12 resta `[ ]`; limiti di Orca 46 + WebKitGTK 2.52.6, non del markup: nessun componente è cambiato):
  1. Orca non pronuncia il nome di un campo di testo (dice il valore e «selected»): `S4b_text_field_name_is_spoken_by_orca`.
  2. Orca non pronuncia nessuna regione viva, `role="alert"` compreso (dieci forme provate): `S6b_notice_is_spoken_by_orca`. La
     strada «provare `alert` in B5» che questo handoff indicava prima era sbagliata: la conferma si sente perché il fuoco ci entra.
  `S4b` e `S6b` non sono più `it.fails`: la preparazione è asserita e solo l'attesa del parlato è rovesciata (`rejects.toThrow`,
  «buco noto»); due prove di controllo (`control_orca_does_not_say_the_name_of_a_bare_text_field`,
  `control_orca_does_not_say_any_form_of_live_region`) mettono nella pagina elementi nudi e asseriscono ciò che Orca dice oggi, così il
  giorno che cambia la prova lo dice. Serve una decisione (Aitano e Peppe) prima del piano di B3: come far arrivare gli esiti a
  Orca (segnale AT-SPI `announcement`, notifiche del desktop, limite dichiarato), e se confermarli su una sessione GNOME vera.
- Dopo la review di B2: `AboutDialog` usava la classe `facts` (sparita con `styles.css`): ora `design/Facts.css` (`.gx-facts`, la
  griglia di prima di `StatusView`) per le due; `overlays.css` aveva `padding: … 10px` fuori scala: ora `--sp-5`, e
  `css-rules.test.ts::spacing_uses_the_scale` controlla `padding*`, `margin*` e `gap` (eccezione per nome: `margin: -1px` di
  `.visually-hidden`).
- L'interruttore è un `toggle button` per AT-SPI: Orca dice «pressed/not pressed». I gruppi della lista sono `panel`.
- Nelle prove sul menu aspettare il fuoco *dentro* il menu (`menuOpen`) prima di premere `Home`/`End`/`Invio`: il fuoco entra un
  istante dopo la comparsa del menu e un tasto troppo presto va al pulsante.
- Tutto il resto del resoconto di B2 (tabelle U1, G8, G12, Q12, S8, trascrizione di Orca, numeri, «Non verificato») è in
  `docs/08-esito-fase2.md`, «Tastiera, fumo, screen reader (T7)».
