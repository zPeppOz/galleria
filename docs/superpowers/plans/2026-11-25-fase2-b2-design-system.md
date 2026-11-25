# Fase 2 · Blocco B2 — Design system e guscio: piano di implementazione

> **Per chi esegue (agente):** SUB-SKILL RICHIESTA: usa superpowers:subagent-driven-development (consigliata) oppure
> superpowers:executing-plans per eseguire il piano un task alla volta. I passi usano le caselle (`- [ ]`).

**Obiettivo:** il design system di Galleria (token, temi, componenti di base) e il guscio dell'app con lista e
dettaglio, navigabile da tastiera con dati finti, in inglese e in italiano. Chiude **G8** e la parte di **G12** che
riguarda B2, verifica **Q12** al primo task e scioglie **S8**.

**Architettura:** tutto in `ui/`: `src/design/` (token, temi, componenti), `src/i18n/` (cataloghi), `src/shell/`
(guscio). Il lato Rust non cambia, salvo l'elenco dei codici propri della GUI e una prova che legge i cataloghi.
Nessun comando Tauri nuovo, nessun permesso nuovo, nessuna modifica a demone, IPC e profili.

**Stack:** quello di B1 (React 19, TypeScript, Vite, Vitest, Testing Library, axe, WebdriverIO) più
`@base-ui/react` 1.9.0 (MIT) **[V: registro npm, 2026-11-25]**. Per la verifica con lo screen reader: Orca 46.1 e
AT-SPI nell'immagine `galleria-gui-dev` **[V: installati e avviati nell'immagine oggi, vedi «Cosa ho verificato»]**.

**Spec:** `docs/superpowers/specs/2026-11-25-fase2-gui-design.md`, §11, §12, §14, §16 («spec §n»). Stato di partenza:
`docs/08-esito-fase2.md`, sezione «B1». Mockup: `design/mockup/Rubrica.html`. Decisioni D1–D32 e Q12–Q21: chiuse.

**Prima di partire:** le questioni Q22–Q26 (in fondo) devono avere una risposta. Se nessuno risponde vale la
raccomandazione scritta lì: il piano è scritto su quelle. Worktree dedicato a partire da `main` (`1b3d588` o
successivo), che deve contenere questo piano.

Legenda: **[V]** verificato oggi, con la fonte · **[S]** supposto, con il task che lo verifica.

## Criteri di uscita del blocco

| # | Criterio | Come si verifica | Task |
|---|---|---|---|
| U1 | Guscio navigabile con dati finti, nelle due lingue (spec §15) | fumo nella finestra vera: lista, dettaglio, schede, menu, finestra modale e avviso, in `en` e in `it`; la scritta «Dati di esempio» è visibile | T6, T7 |
| G8 | Ogni codice d'errore del demone ha testo inglese e italiano; cambiando lingua l'interfaccia cambia senza riavvio | prova Rust `tests/catalogs.rs` sui due cataloghi; prove dei componenti e fumo sul cambio di lingua | T3, T7 |
| G12 (parte di B2) | Guscio e componenti di base si usano tutti da tastiera; finestra modale, menu, schede, lista e interruttore hanno i ruoli corretti | axe e prove di tastiera su ogni componente e sul guscio; trascrizione di Orca e albero AT-SPI nella finestra vera, annotati nell'esito | T1, T4–T7 |
| Q12 | `@base-ui/react` regge finestra modale e menu con lo screen reader, oppure si è passati a Radix | tabella di accettazione del T1 | T1 |
| S8 | `oklch()` e `color-mix()` disegnati giusti da WebKitGTK | fumo: i pixel dei token contro la tabella esadecimale di questo piano | T2 |

G12 per i componenti dei blocchi successivi (rubrica vera, import, credenziali) resta a quei blocchi.

## Vincoli globali

Valgono tutti i vincoli del piano di B1 (un task = un commit, messaggi in inglese minuscolo imperativo, TDD, nessun
push, niente `unwrap()` fuori dalle prove, la GUI non esegue processi). In più:

- **Non si toccano**: tutti i crate tranne `crates/galleria-gui`; in `galleria-gui` solo `src/error.rs`,
  `tests/catalogs.rs` (nuovo) e `tests/config.rs`. `design/`, `docs/00`–`07` e la spec non si toccano. Eccezioni
  elencate nei task: `lab/images/gui.Dockerfile`, `scripts/gui.sh`, `.github/workflows/ci.yml`, `ROADMAP.md`.
- **Nessun comando Tauri, permesso o plugin nuovo.** `capabilities/main.json` e `build.rs` restano come sono.
- **CSP invariata** (`default-src 'self'; …`): mai `unsafe-inline`, mai un host. Se un componente non funziona con
  questa CSP si cambia il componente, non la CSP (T1).
- **Dove girano i comandi**: come in B1. Ogni task finisce con i due gruppi verdi (host con
  `--exclude galleria-gui`; `scripts/gui.sh check`), e dal T2 anche `scripts/gui.sh smoke`.
- **Stile**: CSS semplice, un file per componente accanto al componente, solo variabili di `tokens.css`. Nei CSS
  fuori da `tokens.css` nessun colore scritto a mano (`#…`, `rgb(`, `oklch(`) e nessuna misura fuori scala per
  raggi, spazi e caratteri: lo controlla una prova (T2). Nessun framework CSS, nessun CSS-in-JS, nessun attributo
  `style` nei componenti salvo valori calcolati.
- **Colore**: `--accent` non è mai colore di testo; `--line` è solo un separatore; il bordo di un controllo usa
  `--line-strong`. Uno stato non si comunica mai con il solo colore: ha sempre un testo o un nome accessibile.
- **Icone**: SVG scritti da noi dentro `ui/src/design/icons.tsx`, al più una dozzina, `aria-hidden` se accanto c'è
  un testo. Niente pacchetti di icone, niente glifi di testo (`⋯`, `★`, `✓`), niente emoji.
- **Caratteri**: solo quelli del sistema. Nessun file di caratteri nel repo, nessuna richiesta di rete.
- **Testi**: solo nei cataloghi `ui/src/i18n/{en,it}.json` (dal T3), mai dentro un componente. I componenti di
  `design/` ricevono i testi come proprietà.
- **Dipendenze JS di produzione**: licenza nell'elenco di `deny.toml`; lo controlla `scripts/check-js-licenses.mjs`
  (T1). Dipendenze di sola prova: anche MPL-2.0 (axe).
- **API di Base UI**: nomi e proprietà dalla documentazione **della versione nel lockfile**. I nomi di questo piano
  valgono per il nostro codice. I nostri componenti espongono proprietà nostre: nessun file fuori da
  `ui/src/design/` importa `@base-ui/react`. Lo controlla una prova (T1), ed è ciò che rende possibile il cambio
  di libreria senza toccare altro.
- Se un punto **[S]** risulta falso si applica il ripiego scritto nel task e lo si annota in
  `docs/08-esito-fase2.md`, sezione «B2». Se il ripiego tocca una decisione D o Q chiusa, ci si ferma e si riporta.

## Punti di attenzione per la revisione

1. **La CSP e gli stili in linea.** `default-src 'self'` vale anche per gli stili: un elemento `<style>` inserito
   da una libreria viene bloccato, mentre le proprietà impostate via script (`element.style`) no. Base UI ha un
   `csp-provider` tra gli `exports` **[V: registro npm]**, segno che in qualche punto inserisce stili: quali
   componenti lo facciano non l'ho verificato. → T1 `no_csp_violation_while_using_dialog_and_menu`, nella finestra vera.
2. **jsdom non basta per l'accessibilità.** Non ha disposizione, colori calcolati né albero di accessibilità: le
   prove Vitest dicono che ruoli e tastiera sono giusti nel DOM, non che lo screen reader li legga. Per questo il
   T1 e il T7 passano dalla finestra vera con Orca.
3. **Orca senza gestore di finestre.** Sotto Xvfb non c'è un gestore di finestre: Orca potrebbe scartare gli
   eventi di una finestra che non risulta attiva **[S]**. Il T1 ha la scala dei ripieghi.
4. **Contrasto.** I numeri della tabella dei token li ho calcolati io con uno script (conversione OKLCH → sRGB e
   formula WCAG 2). Il T2 li rifà dentro una prova che legge `tokens.css`: se un token cambia, la prova lo vede.
5. **Dati finti nell'app vera.** Il guscio di B2 mostra profili inventati. Devono essere riconoscibili
   (scritta fissa «Dati di esempio»), non devono mai chiamare il demone e spariscono con il primo task di B3 →
   T6 `demo_banner_is_visible`, `shell_never_calls_the_daemon`.
6. **Niente regressioni su G1 e G2.** Le cinque prove del fumo di B1 restano e restano verdi; i testi che cercano
   («Start the service», `systemctl`, «Disconnected», «Connected to the service») restano nei cataloghi.
7. **Cambio di lingua a caldo.** Nessun testo va tenuto in uno stato o in una costante di modulo: si traduce
   mentre si disegna → T3 `switching_language_changes_every_visible_text`.

## Token: valori ambigui del mockup e proposta

Il mockup ha 16 variabili (`Rubrica.html`, righe 8–19). Il resto è scritto a mano dentro le regole. Contrasto
calcolato sulle tre superfici (`--bg`, `--panel`, `--panel-2`); riporto **il caso peggiore**. Soglie WCAG 2.2 AA:
4,5 per il testo, 3 per i segni grafici e i bordi dei controlli (1.4.3 e 1.4.11).

### Colori

| # | Cosa non torna nel mockup | Misura | Proposta (vale per i due temi) | Dopo |
|---|---|---|---|---|
| C1 | `--faint` usato come **testo** a 11–12 px (titoli dei gruppi, etichette dello stato, suggerimenti, `kbd`, orari del log) | scuro 3,02 · chiaro 2,47: sotto 4,5 | `--faint`: scuro `#8e8f8c`, chiaro `#696862`. Per tenere la distanza dal livello sopra, `--muted`: scuro `#a6a6a2`, chiaro `#55544f` | faint 4,82 · 4,77; muted 6,42 · 6,48 |
| C2 | Nel tema chiaro `--ok`, `--warn`, `--err`, `--wg`, `--ovpn` **non sono ridefiniti**: restano quelli chiari del tema scuro | come testo 1,61–2,68; sigla del protocollo sulla sua tinta 1,72–2,08 | chiaro: `--ok: oklch(0.47 0.13 150)`, `--warn: oklch(0.48 0.095 75)`, `--err: oklch(0.5 0.19 27)`, `--wg: oklch(0.46 0.075 215)`, `--ovpn: oklch(0.48 0.14 40)`. Scuro: invariati | testo 5,48–5,95; sigla sulla tinta 4,82 |
| C3 | Tema chiaro, pulsante primario: `--accent-ink: #fff` su `--accent` | 3,83: sotto 4,5 (testo a 14 px) | `--accent-ink: #1a1206` anche nel chiaro: l'arancione resta quello del mockup | 4,84 (scuro 8,36, invariato) |
| C4 | `--accent` come colore di testo nel tema chiaro | 3,27: basta per segni e fuoco (3), non per il testo | regola: `--accent` mai come testo; resta per riempimenti, indicatore della scheda, anello del fuoco, stella | fuoco: scuro 7,07 · chiaro 3,27 |
| C5 | `--line` è l'unico bordo, anche per campi, selettore e interruttore spento | 1,16–1,38: sotto 3 | `--line` resta per i separatori. Nuovo `--line-strong` per il bordo dei controlli e della finestra modale: scuro `#73757e`, chiaro `#85837b` | 3,42 · 3,24 |
| C6 | Velo e ombre scritti a mano (`#0008`, `#0009`, `#000a`), uguali nei due temi | — | token `--scrim: #0008`, `--shadow-md`, `--shadow-lg`, uguali nei due temi; la finestra modale ha il bordo `--line-strong` | finestra sul velo: 4,32 (scuro, bordo) · 4,68 (chiaro, riempimento) |
| C7 | Tinte fatte con `color-mix(in oklch, X 14%, transparent)`, senza token | con C2: sigla 5,25 (scuro) · 4,82 (chiaro) | token `--tint: 14%`. Negli avvisi in linea il testo è `--ink` sulla tinta e il colore dello stato va solo all'icona e al bordo | `--ink` sulla tinta ≥ 9,81; icona ≥ 4,16 |
| C8 | Il pallino dello stato distingue acceso, in corso ed errore **solo con il colore**; quello spento è `--faint` al 50% | sotto 3 | fuori da B2 (gli stati veri arrivano in B5). Regola già nei vincoli: accanto al pallino c'è sempre il testo dello stato | — |

Valori esadecimali equivalenti, calcolati (servono al T2 per la prova di S8 e come ripiego di S8). Tutti dentro il
gamut sRGB:

| Token | Scuro | Chiaro |
|---|---|---|
| `--accent` | `oklch(0.76 0.14 62)` = `#f09b48` | `oklch(0.62 0.15 55)` = `#c9690c` |
| `--ok` | `oklch(0.76 0.15 150)` = `#61cb7c` | `oklch(0.47 0.13 150)` = `#006e30` |
| `--warn` | `oklch(0.8 0.14 85)` = `#e7b643` | `oklch(0.48 0.095 75)` = `#7c550f` |
| `--err` | `oklch(0.68 0.19 25)` = `#f75d59` | `oklch(0.5 0.19 27)` = `#b7191c` |
| `--wg` | `oklch(0.74 0.11 200)` = `#3ebfc6` | `oklch(0.46 0.075 215)` = `#146272` |
| `--ovpn` | `oklch(0.75 0.12 40)` = `#ee9373` | `oklch(0.48 0.14 40)` = `#9c3a11` |

Superfici, `--line` e `--ink` restano quelli del mockup (`--ink`: 13,15 nello scuro, 14,57 nel chiaro).
Alternativa a C3 che ho misurato e scartato: accento più scuro, `oklch(0.56 0.139 55)` = `#b05a00`, con il testo
bianco (4,88). Cambia il colore del marchio nel tema chiaro e porta l'accento fuori da quello che il mockup mostra.

### Raggi, spazi, caratteri

| # | Cosa non torna nel mockup | Proposta |
|---|---|---|
| R1 | Un solo token (`--r: 8px`) ma sette raggi scritti a mano: 4, 6, 7, 8, 10, 11, 12 px | scala: `--r-xs: 4px` (sigle, `kbd`) · `--r-sm: 6px` (pulsanti, campi, selettore, righe della lista: assorbe 6 e 7) · `--r-md: 8px` (riquadri, avviso a scomparsa) · `--r-lg: 12px` (finestra modale, menu: assorbe 10 e 12) · `--r-full: 999px` (interruttore) |
| P1 | Nessun token per gli spazi; valori da 1 a 28 px, molti dispari (3, 7, 9) | scala `--sp-1`…`--sp-9`: 2, 4, 6, 8, 12, 16, 20, 24, 28 px. I valori del mockup vanno al passo più vicino (7 e 9 → 8, 10 → 12, 14 → 16, 18 → 16, 22 → 24). Altezze: `--h-control: 36px`, `--h-control-lg: 42px` (pulsante di connessione), `--h-row: 44px` (riga della lista) |
| F1 | Otto dimensioni: 10, 11, 12, 12,5, 13, 14, 16, 22 px | sei: `--fs-xs: 11px` · `--fs-sm: 12px` · `--fs-md: 13px` · `--fs-base: 14px` · `--fs-lg: 16px` · `--fs-xl: 22px`. 10 → 11 (sigla del protocollo), 12,5 → 13 (log, come l'editor) |
| F2 | Pesi 550 e 650: esistono solo nei caratteri variabili. Con un carattere statico (400 e 700) le regole CSS li portano tutti e due a 700 | tre pesi: `--fw-regular: 400` · `--fw-medium: 500` · `--fw-strong: 600`. 550 e 650 → 600. Con un carattere statico 500 scende a 400 e 600 sale a 700: la gerarchia resta |
| F3 | `--font` comincia con «Segoe UI Variable Text» (Windows) e non ha `system-ui` | `--font: system-ui, "Segoe UI Variable Text", "Segoe UI", Cantarell, "Ubuntu", "Noto Sans", ui-sans-serif, sans-serif`. Che `system-ui` in WebKitGTK segua il carattere del desktop è **[S]**, T2; se non lo segue si toglie e resta l'elenco del mockup |
| F4 | Tre interlinee (1,45 · 1,6 · 1,7) | due: `--lh: 1.45`, `--lh-mono: 1.6` |

Non ho trovato altre ambiguità: `--mono`, l'anello del fuoco (2 px, distanza 2 px) e la regola per
`prefers-reduced-motion` passano così come sono. Le dimensioni restano in pixel come nel mockup: lo zoom del testo
non è in questo blocco (vedi «Cosa resta fuori»).

## Mappa dei file

```
ui/package.json  pnpm-lock.yaml                                          T1 (dipendenza), T2 (pngjs, di sola prova)
scripts/check-js-licenses.mjs  scripts/tests/check-js-licenses.sh        T1
scripts/gui.sh                                                            T1 (controllo licenze, `a11y`)
lab/images/gui.Dockerfile  lab/tests/gui-a11y.sh  lab/tests/atspi-dump.py T1
ui/e2e/a11y.config.ts  ui/e2e/overlays.a11y.ts  ui/e2e/support.ts        T1 (overlays.a11y.ts anche T7)
ui/src/design/
  Dialog.tsx  Menu.tsx  overlays.css  + .test.tsx  boundary.test.ts      T1 (stile definitivo T5)
  tokens.css  base.css  theme.ts  color.ts  + tokens.test.ts theme.test.ts css-rules.test.ts   T2
  icons.tsx  Button.tsx  Field.tsx  Switch.tsx  Segmented.tsx  Tabs.tsx  + .css + .test.tsx    T4
  Toast.tsx  InlineAlert.tsx  ConfirmDialog.tsx  + .css + .test.tsx      T5
ui/src/app/AppMenu.tsx  AboutDialog.tsx  + .test.tsx                     T1 (lingua T3, tema T3)
ui/src/i18n/en.json  it.json  index.tsx  + catalog.test.ts               T3
ui/src/prefs.tsx  + prefs.test.tsx                                       T3
crates/galleria-gui/src/error.rs  tests/catalogs.rs                      T3
crates/galleria-gui/tests/config.rs                                      T1 (una prova in più)
ui/src/shell/Shell.tsx  TopBar.tsx  ProfileList.tsx  ProfileDetail.tsx
             StatusBar.tsx  ServiceArea.tsx  demo-data.ts  + .css + .test.tsx   T6
ui/e2e/smoke.e2e.ts                                                      T2, T3, T7 (prove nuove; le cinque di B1 restano)
ui/e2e/shell.a11y.ts                                                     T7
docs/08-esito-fase2.md                                                   sezione «B2», aggiornata a ogni task
ROADMAP.md  .claude/handoffs/                                            T7
```

Spariscono: `ui/src/messages.ts`, `ui/src/messages.test.ts`, `ui/src/i18n.tsx` (T3); `ui/src/styles.css` (T6,
quando l'ultima regola è passata ai componenti). `ui/src/screens/FirstRun.tsx` e `StatusView.tsx` restano e
passano ai componenti di `design/` (T5).

---

### Task 1: verifica di Q12 — finestra modale e menu su Base UI, con lo screen reader

**Stima:** 6 h, con un tetto (sotto). **Verifica:** Q12. **Scioglie:** S14, S15, S16 (nuovi, di questo piano).

**File — crea:** `ui/src/design/{Dialog,Menu}.tsx`, `overlays.css`, le loro `.test.tsx`, `boundary.test.ts`,
`ui/src/app/{AppMenu,AboutDialog}.tsx` con le `.test.tsx`, `ui/e2e/{a11y.config.ts,overlays.a11y.ts,support.ts}`,
`lab/tests/{gui-a11y.sh,atspi-dump.py}`, `scripts/check-js-licenses.mjs`, `scripts/tests/check-js-licenses.sh`.
**Modifica:** `ui/package.json`, `pnpm-lock.yaml`, `ui/src/App.tsx`, `ui/src/messages.ts`, `ui/e2e/smoke.e2e.ts`
(solo per usare `support.ts`), `lab/images/gui.Dockerfile` (`orca`, `at-spi2-core`, `speech-dispatcher`),
`scripts/gui.sh` (`check-here` chiama il controllo delle licenze; nuovi `a11y` e `a11y-here`),
`crates/galleria-gui/tests/config.rs`, `docs/08-esito-fase2.md`.

**Produce:**

```tsx
// design/Dialog.tsx — finestra modale. Proprietà nostre, nessun tipo di Base UI esce da qui.
export function Dialog(props: {
  open: boolean; onClose: () => void;
  title: string; description?: string;          // titolo e descrizione collegati con aria-labelledby / aria-describedby
  closeLabel: string;                           // nome accessibile del pulsante di chiusura
  footer?: ReactNode; children: ReactNode;
}): JSX.Element;

// design/Menu.tsx — menu a comparsa su un pulsante.
export type MenuEntry =
  | { kind: "item"; id: string; label: string; onSelect: () => void; danger?: boolean }
  | { kind: "radio"; id: string; label: string; value: string; options: { value: string; label: string }[]; onChange: (v: string) => void }
  | { kind: "separator" };
export function Menu(props: { label: string; trigger: ReactNode; entries: MenuEntry[] }): JSX.Element;
```

Due usi veri, che restano nell'app: nell'intestazione un pulsante «Menu» (`AppMenu`) con il gruppo «Lingua»
(Sistema · English · Italiano, a scelta singola) e la voce «Informazioni su Galleria», che apre `AboutDialog`:
versione dell'app e percorso del socket da `app_info`, versione del demone da `daemon_view`, licenza. In questo
task la lingua scelta vive in uno stato di `App` (il T3 la sposta in `prefs`). Nessun comando nuovo.

`scripts/check-js-licenses.mjs`: esegue `pnpm licenses list --prod --json` in `ui/` (le opzioni esistono **[V:
`pnpm licenses --help`, pnpm 12.15.0]**; la forma del JSON è **[S16]**: si legge sull'uscita vera), legge l'elenco
`allow` di `deny.toml` e fallisce elencando i pacchetti con una licenza fuori elenco o assente.

`scripts/gui.sh a11y`: nell'immagine costruisce l'app come `smoke`, poi lancia `lab/tests/gui-a11y.sh`, che dentro
`dbus-run-session` e `xvfb-run`: accende l'accessibilità (`gsettings set org.gnome.desktop.interface
toolkit-accessibility true`), avvia `at-spi-bus-launcher`, avvia `orca --debug-file=<tmp>/orca.log`, avvia
`tauri-driver`, esegue le prove `ui/e2e/*.a11y.ts`, e alla fine copia in `target/gui-a11y/` il file delle righe
`SPEECH OUTPUT` e l'albero scritto da `atspi-dump.py`. `atspi-dump.py` usa `gi.repository.Atspi` e stampa, per la
finestra di Galleria, una riga per nodo: ruolo, nome, stati.

**Tabella di accettazione di Q12.** Ogni riga si prova a tre livelli: (a) Vitest in jsdom, (b) albero AT-SPI
nella finestra vera, (c) trascrizione di Orca. (a) e (b) sono asserzioni. Per (c) l'asserzione è sui nomi nostri
(titolo, voci); ruolo e stato pronunciati si leggono e si copiano nell'esito, perché le parole esatte di Orca non
le conosco.

| # | Comportamento | (a) jsdom | (b) AT-SPI | (c) Orca |
|---|---|---|---|---|
| D1 | All'apertura il fuoco entra nella finestra modale; ruolo `dialog`, nome = titolo | `dialog_opens_with_focus_inside_and_is_named_by_its_title` | nodo con ruolo `dialog` e nome «About Galleria», un discendente ha lo stato `focused` | il titolo viene pronunciato |
| D2 | `Tab` e `Shift+Tab` non escono; il contenuto dietro non è raggiungibile | `tab_cycles_inside_the_dialog` | i nodi dell'intestazione e del pannello non sono `showing` o non sono nell'albero | — |
| D3 | La descrizione è collegata | `dialog_is_described_by_its_description` | la relazione `described-by` o la descrizione sul nodo | il testo viene pronunciato |
| D4 | `Esc` e il pulsante di chiusura chiudono; il fuoco torna al pulsante che l'ha aperta (qui: il pulsante «Menu») | `escape_closes_and_returns_focus_to_the_opener` | il pulsante «Menu» ha `focused` | «Menu» viene pronunciato |
| M1 | Il pulsante dichiara il menu e il suo stato | `trigger_exposes_haspopup_and_expanded` | ruolo `push button`, stato `expanded` quando è aperto | — |
| M2 | `Invio`, `Spazio` e `↓` aprono; il fuoco va sulla prima voce; ruolo `menu` con nome | `menu_opens_from_the_keyboard_on_the_first_item` | ruolo `menu`, nome «Menu»; la voce «System» ha `focused` | la prima voce viene pronunciata |
| M3 | `↓` `↑` `Home` `End` e la digitazione di una lettera muovono il fuoco | `arrow_home_end_and_typeahead_move_the_focus` | la voce attesa ha `focused` dopo ogni tasto | ogni voce viene pronunciata quando riceve il fuoco |
| M4 | La scelta singola espone il suo stato e cambia con `Invio` | `radio_items_expose_and_change_their_checked_state` | ruolo `radio menu item`; `checked` su una sola voce | lo stato della voce scelta viene pronunciato |
| M5 | `Esc` chiude e riporta il fuoco sul pulsante; una voce scelta chiude il menu | `escape_closes_the_menu_and_returns_focus` | `expanded` assente, pulsante `focused` | — |
| X1 | Nessuna violazione di axe con finestra e menu aperti | `has_no_accessibility_violations` | — | — |
| X2 | Nessuna violazione della CSP | — | `no_csp_violation_while_using_dialog_and_menu` (ascolta `securitypolicyviolation` nella finestra vera) | — |

- [ ] **1. Prove che falliscono.** Le prove della colonna (a) in `Dialog.test.tsx` e `Menu.test.tsx`, più:

  | File | Prova | Asserzione |
  |---|---|---|
  | `design/boundary.test.ts` | `only_the_design_folder_imports_the_headless_library` | nessun file di `ui/src` fuori da `design/` contiene `@base-ui/react` né `radix-ui` |
  | `app/AppMenu.test.tsx` | `about_item_opens_the_about_dialog` | con la sola tastiera: `Tab` fino a «Menu», `Invio`, `End`, `Invio` → finestra con la versione di `app_info` |
  | | `language_choice_changes_the_texts` | scelta «Italiano» → il pulsante si chiama ancora «Menu» e la voce diventa «Informazioni su Galleria» |
  | `tests/config.rs` | `csp_has_no_unsafe_inline` | la CSP non contiene `unsafe-inline` |
  | `scripts/tests/check-js-licenses.sh` | — | con un JSON di prova (variabile `LICENSES_JSON`) che contiene una licenza `GPL-2.0-only` lo script esce diverso da 0 e nomina il pacchetto; con sole MIT e ISC esce 0 |
  | `ui/e2e/overlays.a11y.ts` | una prova per riga della tabella, colonne (b) e (c) | come in tabella |

- [ ] **2.** Falliscono: i componenti non esistono, lo script non esiste, l'immagine non ha Orca.
- [ ] **3. Implementa**, in quest'ordine, annotando ogni esito in `docs/08-esito-fase2.md` (sezione «B2»):
  1. `pnpm --dir ui add @base-ui/react@1.9.0` (versione esatta, senza `^`). Controllo delle licenze e sua prova.
  2. `Dialog` e `Menu` con il minimo di CSS (variabili già presenti in `styles.css`); `AppMenu`, `AboutDialog`.
     Colonna (a) verde.
  3. Immagine e `scripts/gui.sh a11y`. **S14**: Orca nell'immagine vede il contenuto della webview? Annota cosa
     compare in `orca -l` e nell'albero.
  4. Colonne (b) e (c), e X2 (**S15**).
- [ ] **4.** Verde: `scripts/gui.sh check`, `scripts/gui.sh smoke` (le cinque prove di B1), `scripts/gui.sh a11y`
  due volte di fila; poi il gruppo dell'host.
- [ ] **5. Commit:** `add dialog and menu on base ui and verify them with a screen reader`
  (se si è passati a Radix: `add dialog and menu on radix and verify them with a screen reader`).

**Tetto e scala dei ripieghi per il canale di verifica** (non per la libreria):

1. Trascrizione di Orca più albero AT-SPI. È la strada prevista: 2 h al più per farla girare.
2. Se Orca non riceve eventi dalla webview (S14 falsa a metà): si prova con un gestore di finestre minimo
   nell'immagine (`openbox` o `metacity`), 1 h al più.
3. Se resta muto ma l'albero AT-SPI c'è: vale l'albero, colonna (b); la colonna (c) si scrive «non eseguita» e
   passa alla passata a orecchio di Q22.
4. Se non c'è nemmeno l'albero: **stop**. Si committa ciò che è verde in jsdom, si scrive nell'esito la lista di
   controllo manuale (le righe della tabella) e si blocca la card: Q12 non è verificata e i task dal T5 in poi
   non partono.

**Condizione di stop per la libreria.** Base UI è **bocciata** se una riga D1–D4, M1–M5 o X2 fallisce nella
finestra vera e non si corregge in 2 h usando solo proprietà documentate della libreria (niente attributi ARIA
aggiunti a mano sopra i suoi, niente copie modificate, niente `unsafe-inline`). In quel caso, **in questo stesso
task**:

1. si sostituisce `@base-ui/react` con `radix-ui` 1.8.1 (MIT **[V: registro npm]**) dentro `Dialog.tsx` e
   `Menu.tsx`, senza cambiare le loro proprietà né un solo file fuori da `ui/src/design/`;
2. si rilancia la stessa tabella, senza toccarla;
3. si scrive nell'esito quale riga ha bocciato Base UI, con la trascrizione.

Se anche Radix fallisce **la stessa riga**, il difetto è probabilmente di WebKitGTK o di Orca, non della
libreria: **stop**, card bloccata con le due trascrizioni. Non si scrive una finestra modale a mano e non si
allenta la tabella.

**Fatto quando:** la tabella è verde con una delle due librerie e l'esito dice quale; `grep -rn "base-ui\|radix"
ui/src --include=*.tsx --include=*.ts -l` elenca solo file di `ui/src/design/`; S14, S15 e S16 hanno un esito.

---

### Task 2: token e temi (D30, S8)

**Stima:** 5 h. **Scioglie:** S8, S17. **Applica:** la proposta dei token di questo piano (Q23).

**File — crea:** `ui/src/design/{tokens.css,base.css,theme.ts,color.ts}`, `tokens.test.ts`, `theme.test.ts`,
`css-rules.test.ts`. **Modifica:** `ui/src/main.tsx`, `ui/src/styles.css` (via le variabili, restano le regole),
`ui/src/design/overlays.css`, `ui/e2e/smoke.e2e.ts`, `ui/package.json` (`pngjs`, MIT, di sola prova).

**Produce:**

- `tokens.css`: `:root { … }` con il tema scuro, `:root[data-theme="light"] { … }` con il chiaro, `color-scheme`
  impostato nei due. Colori, raggi, spazi, altezze, caratteri: i nomi e i valori delle tabelle sopra, più
  `--tint`, `--scrim`, `--shadow-md`, `--shadow-lg`, `--focus-ring`.
- `base.css`: azzeramento minimo, `body`, `:focus-visible`, `prefers-reduced-motion`, la classe `.visually-hidden`.
- `theme.ts`, puro:

```ts
export type ThemeChoice = "system" | "light" | "dark";
export type Theme = "light" | "dark";
export function resolveTheme(choice: ThemeChoice, systemPrefersLight: boolean): Theme;   // system → chiaro solo se il sistema lo chiede
export function applyTheme(root: HTMLElement, theme: Theme): void;                       // imposta data-theme, sempre esplicito
export function watchSystemTheme(onChange: (prefersLight: boolean) => void): () => void; // matchMedia; restituisce la funzione che toglie l'ascolto
```

- `main.tsx` applica il tema **prima** di montare React, così non si vede il tema sbagliato per un istante.
- `color.ts` (solo per le prove): lettura di un colore `#rrggbb` o `oklch(L C h)`, conversione in sRGB, rapporto
  di contrasto WCAG, composizione di una tinta sopra una superficie.

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `tokens.test.ts` | `both_themes_define_the_same_tokens` | stessi nomi nei due blocchi, salvo quelli uguali per costruzione (raggi, spazi, caratteri) |
  | | `text_tokens_reach_4_5_on_every_surface` | parametrica sui due temi: `--ink`, `--muted`, `--faint`, `--ok`, `--warn`, `--err`, `--wg`, `--ovpn` su `--bg`, `--panel`, `--panel-2` |
  | | `accent_ink_reaches_4_5_on_accent` | nei due temi |
  | | `control_borders_and_focus_ring_reach_3` | `--line-strong` e `--accent` sulle tre superfici |
  | | `protocol_labels_reach_4_5_on_their_tint` | `--wg` e `--ovpn` sulla propria tinta a `--tint` sopra le tre superfici |
  | | `ink_reaches_4_5_on_status_tints` | `--ink` sulla tinta di `--ok`, `--warn`, `--err` |
  | | `oklch_tokens_match_the_hex_table` | i dodici valori della tabella di questo piano, a ±1 per canale |
  | `theme.test.ts` | `system_choice_follows_the_system_and_defaults_to_dark` | `resolveTheme("system", false) === "dark"`, `("system", true) === "light"` |
  | | `explicit_choice_wins_over_the_system` | — |
  | | `apply_sets_data_theme_on_the_root` | — |
  | | `watch_reports_changes_and_stops_when_asked` | con un `matchMedia` finto |
  | `css-rules.test.ts` | `no_raw_colour_outside_tokens_css` | nei `.css` di `ui/src` diversi da `tokens.css`: nessun `#rgb`, `rgb(`, `hsl(`, `oklch(` |
  | | `accent_is_never_a_text_colour` | nessuna dichiarazione `color: var(--accent)` |
  | | `radius_and_font_size_use_the_scale` | `border-radius` e `font-size` hanno solo `var(--r-*)` e `var(--fs-*)` (o `inherit`, `0`, `50%`). Unica eccezione, scritta per nome nella prova: `styles.css`, che ha ancora le misure di B1 e sparisce nel T6 insieme all'eccezione |
  | `e2e/smoke.e2e.ts` | `tokens_paint_the_expected_srgb` | nella finestra vera, per ogni token colorato e per i due temi: un riquadro con quello sfondo, immagine dell'elemento, pixel centrale uguale all'esadecimale della tabella a ±3 per canale; lo stesso per una tinta al 14% |
  | | `theme_attribute_is_set_before_first_paint` | appena la finestra risponde, `document.documentElement.dataset.theme` è `dark` o `light` |

- [ ] **2.** Falliscono. Nota: `text_tokens_reach_4_5_on_every_surface` **deve** fallire se si mettono in
  `tokens.css` i valori del mockup così come sono (C1 e C2): farlo una volta, a mano, per vedere che la prova morde.
- [ ] **3. Implementa.** **S8**: se `tokens_paint_the_expected_srgb` fallisce per i colori `oklch()` o per
  `color-mix()`, i token passano ai valori esadecimali della tabella (e le tinte a un colore pieno calcolato per
  superficie), e lo si scrive nell'esito. **S17**: nell'immagine, annota quale carattere risolve `system-ui`
  (`fc-match` e lo stile calcolato); se non è il carattere del sistema, applica il ripiego di F3.
- [ ] **4.** `scripts/gui.sh check`, `scripts/gui.sh smoke`; gruppo dell'host.
- [ ] **5. Commit:** `add design tokens with light and dark themes and contrast tests`

**Fatto quando:** nessuna variabile di colore è definita fuori da `tokens.css`; S8 ha un esito per Ubuntu 24.04
(l'immagine). Per Ubuntu 26.04 e Fedora S8 resta aperto fino alla prima passata manuale su quelle distribuzioni
(B6, con G10): lo si scrive nell'esito, non lo si dichiara chiuso.

---

### Task 3: cataloghi `en` e `it`, prova dei codici, lingua a caldo (D22, G8)

**Stima:** 5 h. **File — crea:** `ui/src/i18n/{en.json,it.json,index.tsx,catalog.test.ts}`, `ui/src/prefs.tsx`,
`ui/src/prefs.test.tsx`, `crates/galleria-gui/tests/catalogs.rs`. **Modifica:** `crates/galleria-gui/src/error.rs`,
`ui/src/App.tsx`, `ui/src/app/AppMenu.tsx`, tutti i file che importano `./messages` o `./i18n`,
`ui/tsconfig.json` (`resolveJsonModule`), `ui/e2e/smoke.e2e.ts`. **Elimina:** `ui/src/messages.ts`,
`ui/src/messages.test.ts`, `ui/src/i18n.tsx`.

**Produce:**

- Cataloghi JSON piatti, stesse chiavi. Le chiavi di B1 restano uguali. Nuove: `error.<codice>` per ogni codice
  di `ErrorCode` e di `SessionErrorCode` (11 e 8 **[V: `crates/galleria-ipc/src/error.rs`]**; i due insiemi non
  hanno nomi in comune **[V]**), più i testi di menu, finestra e tema. Per le frasi degli errori si parte dalla
  tabella della CLI (`crates/galleria-cli/src/messages.rs`), così CLI e GUI dicono la stessa cosa.
- `i18n/index.tsx`:

```tsx
export type Lang = "en" | "it";
export type MessageKey = keyof typeof import("./en.json");
export function detectLang(): Lang;                                  // navigator.language, inglese come ripiego
export function I18n(props: { lang: Lang; children: ReactNode }): JSX.Element;
export function useT(): (key: MessageKey, vars?: Record<string, string | number>) => string;
export function hasKey(key: string): key is MessageKey;
/** D22: il testo del catalogo per il codice; se il codice non c'è, il messaggio inglese del demone. */
export function errorText(t: ReturnType<typeof useT>, code: string, fallback: string): string;
```

- `prefs.tsx`: `Preferences` (contesto) con `{ lang: "system" | Lang, theme: ThemeChoice }`, `usePrefs()` e
  `setLang`, `setTheme`. Solo in memoria (Q25). Applica `document.documentElement.lang` e il tema. `AppMenu`
  guadagna il gruppo «Tema» (Sistema · Chiaro · Scuro) e legge la lingua da qui.
- `error.rs`: `pub const GUI_CODES: &[&str] = &[];` con il commento che dice la regola: ogni codice che un comando
  della GUI può restituire (prefisso `gui_`, e da B4 i codici `import_*`) si aggiunge qui nello stesso commit che
  lo introduce.

- [ ] **1. Prove che falliscono.**

  `crates/galleria-gui/tests/catalogs.rs` (legge `../../ui/src/i18n/{en,it}.json` con `serde_json`):

  | Prova | Asserzione |
  |---|---|
  | `every_daemon_error_code_has_text_in_both_catalogues` | per ogni variante di `ErrorCode`: `error.<as_str()>` presente e non vuota in `en` e in `it` |
  | `every_session_error_code_has_text_in_both_catalogues` | lo stesso per `SessionErrorCode`, con il nome serializzato |
  | `every_gui_code_has_text_in_both_catalogues` | per ogni voce di `GUI_CODES` |
  | `every_state_and_link_state_has_a_label` | `state.<s>` per le sette varianti di `State`; `bar.<l>` per le cinque di `LinkState` |
  | `no_error_key_is_left_without_a_code` | ogni chiave `error.*` dei cataloghi corrisponde a un codice di uno dei tre elenchi |
  | `gui_code_literals_are_registered` | ogni letterale `"gui_…"` o `"import_…"` trovato nei sorgenti di `crates/galleria-gui/src` sta in `GUI_CODES` |
  | `the_checker_reports_a_missing_code` | la funzione di confronto, data un catalogo senza `error.internal`, restituisce `["error.internal"]` |

  `ErrorCode` e `SessionErrorCode` non hanno un elenco delle varianti e `galleria-ipc` non si tocca: la prova
  tiene un suo array e, accanto, una funzione con un `match` **senza ramo di ripiego** su ogni enum. Una variante
  nuova non compila finché non la si aggiunge all'array, e l'array senza testo fa fallire la prova. Lo stesso
  per `State` e per `LinkState`.

  | File | Prova | Asserzione |
  |---|---|---|
  | `i18n/catalog.test.ts` | `every_key_exists_in_both_languages` · `has_no_empty_text` | come in B1, sui file JSON |
  | | `placeholders_match_between_languages` | per ogni chiave, lo stesso insieme di `{nome}` in `en` e in `it` |
  | | `unknown_error_code_falls_back_to_the_daemon_message` | `errorText(t, "brand_new_code", "Daemon said so") === "Daemon said so"` |
  | | `known_error_code_is_translated` | `auth_failed` in `it` è il testo del catalogo, non il ripiego |
  | `prefs.test.tsx` | `system_language_is_detected_and_english_is_the_fallback` | `navigator.language` `it-IT` → `it`; `fr-FR` → `en` |
  | | `switching_language_changes_every_visible_text` | `App` montata in `it` con il demone simulato fermo; scelta «English» dal menu: nel documento non resta nessun testo del catalogo `it` che sia diverso dal suo corrispondente `en`; nessuno smontaggio di `App` |
  | | `html_lang_follows_the_language` | — |
  | | `theme_choice_from_the_menu_sets_data_theme` | — |
  | `e2e/smoke.e2e.ts` | `language_switches_without_a_restart` | finestra vera senza demone: dal menu «Italiano» → compare «Avvia il servizio»; «English» → «Start the service»; la sessione WebDriver è la stessa |

- [ ] **2.** Falliscono. **3.** Implementa. Dopo lo spostamento, `grep -rn "messages\"" ui/src` non trova nulla.
- [ ] **4.** `scripts/gui.sh check`, `scripts/gui.sh smoke`; gruppo dell'host.
- [ ] **5. Commit:** `move texts to en and it catalogues and test them against the daemon codes`

**Fatto quando:** `cargo test -p galleria-gui --test catalogs` è verde nell'immagine; togliendo a mano una chiave
`error.*` da `it.json` diventa rosso (provato una volta e annotato); le cinque prove di fumo di B1 sono verdi.

---

### Task 4: controlli di base — pulsante, campo, interruttore, selettore, schede, icone

**Stima:** 6 h. **File — crea:** `ui/src/design/{icons,Button,Field,Switch,Segmented,Tabs}.tsx`, un `.css` e una
`.test.tsx` per ciascuno.

**Produce** (proprietà nostre; dietro, Base UI o Radix secondo l'esito del T1, oppure HTML nativo dove basta):

```tsx
export function Button(props: { variant?: "default" | "primary" | "danger" | "ghost"; size?: "md" | "lg";
  icon?: ReactNode; disabled?: boolean; busy?: boolean; onClick?: () => void; children: ReactNode;
  "aria-label"?: string }): JSX.Element;                                 // <button type="button"> nativo
export function Field(props: { label: string; hint?: string; error?: string; mono?: boolean; readOnly?: boolean;
  value: string; onChange?: (v: string) => void;
  as?: "input" | "textarea" | "select"; options?: { value: string; label: string }[];
  type?: "text" | "password"; inputMode?: "numeric" }): JSX.Element;      // etichetta, suggerimento ed errore collegati
export function Switch(props: { label: string; hint?: string; checked: boolean; disabled?: boolean;
  onChange: (v: boolean) => void }): JSX.Element;                         // ruolo switch
export function Segmented(props: { label: string; value: string; options: { value: string; label: string }[];
  onChange: (v: string) => void; readOnly?: boolean }): JSX.Element;      // gruppo a scelta singola (radiogroup)
export function Tabs(props: { label: string; value: string; onChange: (v: string) => void;
  tabs: { id: string; label: string; panel: ReactNode }[] }): JSX.Element;
```

Scelte: l'elenco a tendina è il `<select>` nativo dentro `Field` (accessibile da sé; il mockup usa quello). Il
selettore del mockup usa `aria-pressed` su più pulsanti: qui è un gruppo a scelta singola, che è ciò che è. Le
icone: `more`, `close`, `check`, `warning`, `error`, `info`, `star`, `chevron`, `copy`.

- [ ] **1. Prove che falliscono.** Per ogni componente, in jsdom:

  | Componente | Prove |
  |---|---|
  | tutti | `has_no_accessibility_violations` (axe) · `is_reachable_and_operable_from_the_keyboard` |
  | `Button` | `is_a_native_button_of_type_button` · `disabled_and_busy_do_not_fire_click` · `busy_is_announced` (`aria-busy`) · `icon_only_button_needs_a_label` (senza testo e senza `aria-label` la prova di tipo o di esecuzione fallisce) |
  | `Field` | `label_hint_and_error_are_linked_to_the_control` (`aria-describedby`, `aria-invalid`) · `read_only_field_is_focusable_but_not_editable` · `select_lists_its_options` · `password_field_never_renders_its_value_as_text` |
  | `Switch` | `has_switch_role_and_checked_state` · `space_and_enter_toggle` · `disabled_switch_does_not_toggle` · `label_click_toggles` |
  | `Segmented` | `is_a_radiogroup_with_one_checked_option` · `arrows_move_the_choice` · `is_one_tab_stop` · `read_only_shows_the_value_and_does_not_change` |
  | `Tabs` | `has_tablist_tab_and_tabpanel_roles_linked_together` · `arrows_home_and_end_move_between_tabs` · `only_the_selected_panel_is_rendered` · `tablist_is_one_tab_stop` |
  | `icons` | `icons_are_hidden_from_assistive_technology_by_default` |

- [ ] **2.** Falliscono. **3.** Implementa, con soli token. Bersaglio del puntatore: ogni controllo almeno
  24 × 24 px (WCAG 2.5.8); l'interruttore del mockup è 38 × 22: l'area che riceve il clic si allarga a 24 in altezza.
- [ ] **4.** `scripts/gui.sh check`; gruppo dell'host.
- [ ] **5. Commit:** `add base controls: button, field, switch, segmented control, tabs`

**Fatto quando:** `css-rules.test.ts` (T2) è verde sui file nuovi; nessun componente contiene un testo visibile.

---

### Task 5: avvisi, conferma, stile definitivo di finestra e menu; le schermate di B1 sui componenti

**Stima:** 5 h. **File — crea:** `ui/src/design/{Toast,InlineAlert,ConfirmDialog}.tsx` con `.css` e `.test.tsx`.
**Modifica:** `ui/src/design/{Dialog,Menu}.tsx`, `overlays.css`, `ui/src/screens/{FirstRun,StatusView}.tsx` e le
loro prove, `ui/src/App.tsx`.

**Produce:**

```tsx
export function ToastProvider(props: { closeLabel: string; children: ReactNode }): JSX.Element;
export function useToast(): (message: string, opts?: { tone?: "info" | "ok" | "error" }) => void;   // regione viva: status, o alert per "error"
export function InlineAlert(props: { tone: "info" | "ok" | "warn" | "error"; title?: string;
  children: ReactNode; actions?: ReactNode }): JSX.Element;               // role="alert" solo per "error", altrimenti role="status"
export function ConfirmDialog(props: { open: boolean; title: string; body: string; confirmLabel: string;
  cancelLabel: string; danger?: boolean; onConfirm: () => void; onCancel: () => void }): JSX.Element;  // alertdialog
```

`FirstRun` usa `InlineAlert`, `Button` e un blocco a carattere fisso; `StatusView` usa `InlineAlert` per l'errore e
per i backend non disponibili. Testi, ruoli e comportamento restano quelli di B1: le prove esistenti non si
allentano.

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `Toast.test.tsx` | `message_is_announced_in_a_live_region` | il testo compare dentro un elemento `status` già presente nel documento prima del messaggio |
  | | `error_tone_uses_an_assertive_region` | — |
  | | `toast_leaves_by_itself_and_pauses_while_focused_or_hovered` | con il tempo finto di Vitest |
  | | `toast_can_be_dismissed_from_the_keyboard` | il pulsante di chiusura ha il nome passato; il fuoco non viene mai spostato sull'avviso da solo |
  | `InlineAlert.test.tsx` | `error_is_an_alert_and_the_other_tones_are_status` · `tone_is_not_conveyed_by_colour_alone` (icona con nome accessibile per tono) |
  | `ConfirmDialog.test.tsx` | `has_alertdialog_role_named_by_title_and_described_by_body` · `focus_starts_on_cancel_when_danger` · `escape_cancels` · `confirm_calls_once` |
  | `Dialog.test.tsx`, `Menu.test.tsx` | le prove del T1, invariate | restano verdi dopo il cambio di stile |
  | `FirstRun.test.tsx`, `StatusView.test.tsx` | le prove di B1, invariate | restano verdi |
  | tutti | `has_no_accessibility_violations` | — |

- [ ] **2.** Falliscono le nuove. **3.** Implementa. La finestra modale prende `--r-lg`, `--shadow-lg`, bordo
  `--line-strong`, velo `--scrim`; il menu `--r-lg` e `--shadow-md`.
- [ ] **4.** `scripts/gui.sh check`, `scripts/gui.sh smoke`, `scripts/gui.sh a11y` (la tabella del T1 deve
  restare verde con lo stile nuovo); gruppo dell'host.
- [ ] **5. Commit:** `add toast, inline alert and confirm dialog and restyle the b1 screens`

**Fatto quando:** nessun file di `ui/src/screens/` usa una classe di `styles.css`.

---

### Task 6: guscio — lista e dettaglio con dati finti

**Stima:** 6 h. **File — crea:** `ui/src/shell/{Shell,TopBar,ProfileList,ProfileDetail,StatusBar,ServiceArea}.tsx`,
`demo-data.ts`, i `.css` e le `.test.tsx`. **Modifica:** `ui/src/App.tsx`, `ui/src/App.test.tsx`. **Elimina:**
`ui/src/styles.css`.

**Produce:** la disposizione del mockup dentro la finestra con le decorazioni del sistema (Q24):

```
┌ TopBar: «Galleria» · [Dati di esempio] ·························· [Menu] ┐
├ aside (300 px) ────────┬ main ───────────────────────────────────────────┤
│ ProfileList            │ ServiceArea   primo avvio · backend assenti · stato vero del demone (Q26)
│  gruppi e righe        │ ProfileDetail intestazione · [Altre azioni] · striscia di stato
│                        │               Tabs: Generale · Autenticazione · Rete · Configurazione · Log
├────────────────────────┴─────────────────────────────────────────────────┤
└ StatusBar: stato della connessione ·············· collegamento · versione ┘
```

- `demo-data.ts`: cinque profili inventati (quelli del mockup: nomi, protocollo, gruppo, server su domini
  `example`), come costante tipata `DemoProfile[]`. Nessun segreto, nemmeno finto.
- `ProfileList`: `listbox` con un `group` per gruppo (nome accessibile = titolo del gruppo), righe `option`,
  selezione che segue il fuoco, un solo punto di sosta del `Tab`. Riceve i dati dalle proprietà: B3 gli darà quelli veri.
- `ProfileDetail`: intestazione (nome come titolo di livello 1 del contenuto, sigla del protocollo con testo
  «WireGuard» o «OpenVPN» per lo screen reader, gruppo), menu «Altre azioni» (Duplica · Esporta · Elimina), striscia
  di stato con quattro celle vuote («—»), cinque schede. «Generale» mostra un modulo di esempio con `Field`,
  `Field as="select"`, `Segmented` in sola lettura e `Switch` per `auto_reconnect` (i campi di D26, senza quelli
  tolti dalla spec §16). Le altre quattro schede mostrano una riga sola: `pane.arrives_later`.
- Le azioni non fanno nulla di vero: «Elimina» apre `ConfirmDialog` e, alla conferma, mostra l'avviso a scomparsa
  `demo.no_change` («Dati di esempio: nessuna modifica»). «Duplica» ed «Esporta» mostrano lo stesso avviso.
- `ServiceArea` (Q26): con `link !== "connected"` il pannello di `FirstRun`; altrimenti gli avvisi dei backend non
  disponibili e, se lo stato del demone non è `disconnected`, `StatusView`. La lista resta sempre usabile.
- `StatusBar`: a sinistra l'etichetta dello stato vero (`state.<s>`), a destra il collegamento (i testi `bar.*`
  di B1) e la versione dell'app. `aria-live="polite"` come in B1.
- Larghezza sotto 860 px: una colonna sola, la lista sopra il dettaglio (il mockup la nasconde: qui no, o la
  rubrica non si userebbe più).
- **Non c'è il campo di ricerca**: arriva in B3 con la ricerca. Un campo che non filtra sarebbe finto.

- [ ] **1. Prove che falliscono.**

  | File | Prova | Asserzione |
  |---|---|---|
  | `Shell.test.tsx` | `has_one_banner_one_main_and_named_regions` | punti di riferimento: `banner`, `complementary` con nome, `main`, `contentinfo`; un solo titolo di livello 1 |
  | | `demo_banner_is_visible` | il testo `demo.banner` è nel documento, in `en` e in `it` |
  | | `shell_never_calls_the_daemon` | dopo un giro completo di azioni (selezione, schede, menu, elimina con conferma), i soli comandi simulati invocati sono `daemon_view`, `app_info` e, se premuto «Riprova», `daemon_retry` |
  | | `first_run_panel_shows_while_the_list_stays_usable` | con `link = "daemon_not_running"`: c'è il comando `systemctl` e le frecce nella lista cambiano il dettaglio |
  | | `backend_warning_is_shown_when_connected` | `openvpn_not_found` → il testo di B1 |
  | | `daemon_status_shows_when_a_session_exists` | con `status.state = "connected"` simulato compare `StatusView` |
  | | `works_in_both_languages` | parametrica `en`, `it`: ogni testo visibile del guscio viene dal catalogo della lingua (nessuna chiave grezza, nessun testo dell'altra lingua) |
  | `ProfileList.test.tsx` | `is_a_listbox_with_named_groups_and_options` | — |
  | | `arrows_home_end_move_and_select` · `typing_a_letter_jumps_to_the_next_match` · `is_one_tab_stop` | — |
  | | `selected_option_is_exposed` | `aria-selected` su una sola riga |
  | `ProfileDetail.test.tsx` | `protocol_has_a_text_alternative` · `tabs_switch_panels` · `more_actions_menu_lists_three_actions` | — |
  | | `delete_asks_for_confirmation_and_changes_nothing` | conferma → avviso a scomparsa; i profili finti sono ancora cinque |
  | `StatusBar.test.tsx` | `shows_the_real_state_and_link` | parametrica sui cinque `link` |
  | tutti | `has_no_accessibility_violations` | — |

- [ ] **2.** Falliscono. **3.** Implementa.
- [ ] **4.** `scripts/gui.sh check`, `scripts/gui.sh smoke` (le prove di B1 e dei T2–T3 verdi sul guscio nuovo);
  gruppo dell'host.
- [ ] **5. Commit:** `add the list and detail shell with sample data`

**Fatto quando:** `ui/src/styles.css` non esiste più e `css-rules.test.ts` non ha più eccezioni; `grep -rn "invoke(" ui/src/shell` non trova nulla.

---

### Task 7: tastiera sul guscio intero, fumo, screen reader, esito del blocco

**Stima:** 5 h. **File — crea:** `ui/src/shell/keyboard.test.tsx`, `ui/e2e/shell.a11y.ts`. **Modifica:**
`ui/e2e/smoke.e2e.ts`, `ui/e2e/overlays.a11y.ts`, `docs/08-esito-fase2.md`, `ROADMAP.md`, `.claude/handoffs/`.

- [ ] **1. Prove che falliscono.**

  `keyboard.test.tsx`, sul guscio intero, senza mai usare il puntatore:

  | Prova | Asserzione |
  |---|---|
  | `tab_order_follows_the_layout` | l'ordine dei punti di sosta è: «Menu» → lista → «Altre azioni» → schede → primo controllo della scheda → (con il primo avvio visibile) «Copia», «Riprova» nel loro posto |
  | `every_interactive_element_is_reachable` | l'insieme degli elementi raggiunti con `Tab` e frecce è uguale all'insieme degli elementi interattivi del documento |
  | `focus_is_never_lost_to_the_body` | dopo ogni chiusura (menu, finestra, conferma, avviso) `document.activeElement` non è `body` |
  | `a_full_tour_needs_no_pointer` | scelta di un profilo, cambio di scheda, interruttore, «Elimina» con conferma, cambio di lingua e di tema: tutto con la tastiera |
  | `focus_ring_rule_exists_for_every_control` | ogni componente ha una regola `:focus-visible` che usa `--focus-ring` (letta dai `.css`) |

  `e2e/smoke.e2e.ts`, finestra vera:

  | Prova | Asserzione |
  |---|---|
  | `shell_is_navigable_with_sample_data` | con il demone finto: compaiono i cinque profili e «Sample data»; `↓` nella lista cambia il titolo del dettaglio; la scheda «Log» si apre con le frecce |
  | `shell_works_in_both_languages` | dal menu «Italiano»: «Dati di esempio», «Generale», «Altre azioni»; ritorno a «English» |
  | `both_themes_render` | dal menu «Chiaro» e «Scuro»: lo sfondo calcolato di `body` è quello del token nei due casi |
  | `no_csp_violation_during_the_tour` | nessun evento `securitypolicyviolation` durante le tre prove sopra |

  `e2e/shell.a11y.ts` (Orca e AT-SPI, come nel T1): lista (`list box`, nome, voce a fuoco pronunciata), schede
  (`page tab list`, scheda scelta), interruttore (stato pronunciato), conferma di eliminazione (`alert`, titolo e
  testo pronunciati), avviso a scomparsa (pronunciato senza spostare il fuoco).

- [ ] **2.** Falliscono. **3.** Correggi ciò che le prove trovano, nei componenti a cui appartiene.
- [ ] **4.** `scripts/gui.sh check`; `scripts/gui.sh smoke` **tre volte di fila**; `scripts/gui.sh a11y` due volte;
  gruppo dell'host.
- [ ] **5. Esito e roadmap.** In `docs/08-esito-fase2.md`, sezione «B2»: tabella U1, G8, G12, Q12 con la prova che
  li copre e il risultato; S8 e S14–S17 con esito e ripiego; la **trascrizione di Orca** del giro completo (righe
  `SPEECH OUTPUT`, così come sono) e l'albero AT-SPI dei punti della tabella del T1; numero di prove per livello;
  «Non verificato», che contiene almeno: S8 su Ubuntu 26.04 e Fedora, la passata a orecchio (Q22), il tema del
  sistema su un desktop vero, l'aspetto a occhio in una finestra. `ROADMAP.md`: le caselle di «B2» secondo i
  fatti. Handoff in `.claude/handoffs/`.
- [ ] **6. Commit:** `cover the shell with keyboard, smoke and screen reader tests`

**Fatto quando:** i criteri di uscita in cima hanno ciascuno una prova verde, oppure l'esito dice quale non lo è
e perché; nessuna casella della roadmap è spuntata senza la sua prova.

---

## Copertura della spec

| Spec | Dove |
|---|---|
| §12 token dal mockup, tema chiaro e scuro, tema del sistema | T2; scelta manuale T3 (in memoria: il salvataggio in `state.toml` è B3, Q25) |
| §12 `oklch()` (S8) | T2 `tokens_paint_the_expected_srgb` |
| §12 componenti: pulsante, campo, interruttore, selettore a segmenti, schede | T4 |
| §12 finestra modale, menu | T1, stile T5 |
| §12 avviso a scomparsa, avviso in linea | T5 |
| §12 libreria «headless» (Q12) | T1 |
| §12 licenze JS in CI | T1 (`check-here`, quindi anche il job `gui`) |
| §12 niente emoji, SVG nostri | T4 `icons.tsx`; vincoli globali |
| §11 cataloghi, prova Rust dei codici, lingua del sistema, scelta manuale, inglese di ripiego, niente riavvio | T3 |
| §15 disposizione lista + dettaglio, tastiera | T6, T7 |
| §16 differenze dal mockup, punto 5 (barra con lo stato vero) | T6 `StatusBar` |
| §14 livello 1 e livello 3 | ogni task; fumo T2, T3, T7 |
| G8 | T3 (`tests/catalogs.rs`, `switching_language_changes_every_visible_text`), T7 (`shell_works_in_both_languages`) |
| G12, parte di B2 | T1 (finestra e menu), T4–T6 (axe e tastiera per componente), T7 (guscio intero, Orca) |

Punti [S] di questo piano (la numerazione continua S1–S13 della spec; l'esito si scrive in `docs/08`):

| # | Supposizione | La verifica | Se è falsa |
|---|---|---|---|
| S8 | `oklch()` e `color-mix()` disegnati giusti da WebKitGTK | T2, sull'immagine (Ubuntu 24.04, WebKitGTK 2.52.6) | token esadecimali dalla tabella di questo piano |
| S14 | Orca nell'immagine riceve il contenuto e gli eventi della webview | T1 | scala dei ripieghi del T1 |
| S15 | Finestra modale e menu della libreria funzionano con la nostra CSP | T1, riga X2 | `csp-provider` della libreria se basta; altrimenti la libreria è bocciata su X2 |
| S16 | `pnpm licenses list --prod --json` dà nome e licenza di ogni pacchetto di produzione | T1 | lo script legge i `package.json` sotto `node_modules/.pnpm` |
| S17 | `system-ui` in WebKitGTK è il carattere del desktop | T2 | si toglie `system-ui` (F3) |

## Cosa resta fuori da B2, per scelta

- Ricerca, preferiti, ordinamento, gruppi modificabili, dati veri, `state.toml`: B3. Di conseguenza anche la
  scorciatoia `Ctrl+K` e il salvataggio di lingua e tema.
- `Ctrl+O`, finestra di import: B4. Connetti con `Invio`, stati veri, pallino dello stato (C8), credenziali: B5.
- Finestra delle impostazioni: B6. In B2 lingua e tema stanno nel menu.
- Zoom del testo e dimensioni in `rem`: non richiesto da G12; le dimensioni restano in pixel come nel mockup.
- Prova di Orca in CI: resta locale (Q22).
- Confronto a immagini con il mockup: nessuno guarda i pixel dell'insieme, solo i colori dei token.

## Cosa ho verificato e cosa no

Verificato oggi (2026-11-25):

- `@base-ui/react` 1.9.0: licenza MIT, `exports` con `dialog`, `alert-dialog`, `menu`, `tabs`, `switch`, `toast`,
  `field`, `csp-provider`; richiede `react` 17–19. `radix-ui` 1.8.1: MIT. (registro npm)
- Il menu di Base UI ha gruppi a scelta singola (`Menu.RadioGroup`, `Menu.RadioItem`). (pagina «Menu» della
  documentazione, letta oggi; è quella dell'ultima versione, non necessariamente della 1.9.0)
- Nell'immagine `galleria-gui-dev`: `apt-get install orca at-spi2-core speech-dispatcher` va a buon fine
  (Orca 46.1, AT-SPI 2.52.0); Orca parte sotto `dbus-run-session` e `xvfb-run`, vede un'applicazione GTK
  (`orca -l` elenca `zenity`) e scrive righe `SPEECH OUTPUT` nel file di `--debug-file`. Ho provato con una
  finestra di `zenity`, **non con la webview di Galleria**: che Orca legga il contenuto di WebKitGTK è S14.
- `pnpm licenses` ha le opzioni `--prod` e `--json` (pnpm 12.15.0).
- Contrasti e valori esadecimali della sezione dei token: calcolati con uno script mio (non è nel repo); la prova
  del T2 li ricalcola.
- 11 varianti di `ErrorCode`, 8 di `SessionErrorCode`, 7 di `State`, 5 di `LinkState`; nessun codice `gui_*`
  esiste ancora nel codice.

Non verificato:

- Nessuna riga di codice di B2 è stata scritta o eseguita: componenti, prove e script di questo piano sono progetto.
- Il comportamento di Base UI con lo screen reader e con la CSP: è il T1.
- Le parole che Orca pronuncia per ruoli e stati.
- Come WebKitGTK riporta il tema del sistema (`prefers-color-scheme`) su un desktop vero.
- La regola sui pesi dei caratteri (F2) l'ho applicata dalla specifica CSS a memoria, senza una prova nella webview.

## Questioni aperte nuove

La numerazione continua Q12–Q21 della spec. Ognuna ha la mia raccomandazione, che vale come **predefinito per chi
implementa** se nessuno decide prima del task indicato.

| # | Questione | Alternative | Raccomandazione | Serve entro |
|---|---|---|---|---|
| Q22 | Chi fa la verifica con lo screen reader, e dove. Peppe lavora su WSL, dove Orca non c'è; G12 chiede una passata manuale annotata | (a) trascrizione di Orca e albero AT-SPI nell'immagine, prodotti dalle prove e copiati nell'esito · (b) una persona con Orca su un desktop Linux vero, prima di B3 · (c) solo jsdom e axe | **(a) come prova di Q12 e di G12 in B2**, ripetibile e senza macchine in più. La passata **a orecchio** su un desktop vero si fa una volta sola in B6, insieme alle prove manuali di G10 su Ubuntu e Fedora, e fino ad allora sta in «Non verificato». La prova di Orca resta locale, non in CI: allunga l'immagine e il fumo, e non so ancora quanto sia stabile | T1 |
| Q23 | Correzioni ai token del mockup (tabelle sopra): C1–C7, R1, P1, F1–F4 | approvarle tutte · tenere i valori del mockup dove non passano AA · per C3, accento più scuro con testo bianco | **Approvarle tutte.** Senza C1, C2, C3 e C5 il tema chiaro non passa AA e nel tema scuro non lo passano i testi piccoli. A occhio cambia poco nello scuro (grigi un po' più chiari, bordo dei campi più visibile) e molto nel chiaro (colori di stato scuri, testo scuro sul pulsante arancione) | T2 |
| Q24 | Barra del titolo: il mockup ne disegna una sua, con i pulsanti della finestra | (a) decorazioni del sistema, e sotto una barra nostra con nome e menu · (b) barra disegnata da noi, senza decorazioni | **(a)**. Con (b) servono permessi nuovi per la webview (trascinare, ridurre, chiudere la finestra), che in B1 abbiamo tenuto a cinque, e su GNOME, KDE e Windows la barra va rifatta tre volte. Dal mockup spariscono i tre pulsanti della finestra e i pulsanti dimostrativi «Tray» e «Tema» | T6 |
| Q25 | Dove si salvano lingua e tema scelti a mano. La spec mette il tema in `state.toml` (B3) e la lingua nelle impostazioni (B6) | (a) in B2 solo in memoria; in B3 tutti e due in `state.toml` con `ui_state_get` e `ui_state_set` · (b) `localStorage` della webview da subito · (c) un comando nuovo in B2 | **(a)**. (b) crea un secondo posto dove vive lo stato, in una cartella che dipende dall'identificatore dell'app, da migrare dopo. (c) porta in B2 un pezzo di B3. Il costo: fino a B3 la scelta si perde alla chiusura | T3 |
| Q26 | Dove finisce nel guscio la vista di B1 (stato vero del demone), visto che lista e dettaglio mostrano dati finti | (a) una fascia in cima al dettaglio: primo avvio, backend assenti e, solo se c'è una sessione, lo stato del demone; la barra in basso dice sempre stato e collegamento · (b) una riga fissa «Servizio» in cima alla lista · (c) toglierla fino a B5 | **(a)**. Non perde nulla di G1 e G2, tiene la lista usabile anche senza demone (come chiede la spec §5 da B3), e non inventa una riga della lista che B5 dovrebbe poi trasformare nella riga della sessione estranea (D21). Con (c) B2 farebbe un passo indietro rispetto a B1 | T6 |
