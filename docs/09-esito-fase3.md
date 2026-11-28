# Esito della fase 3 — Galleria su Windows

Stato: **P1 in corso** (il solo blocco approvato: Q60 = (a), Peppe, 2026-11-27). Niente è ancora stato eseguito.

Spec: `docs/superpowers/specs/2026-11-26-fase3-windows-design.md`. Piano di P1:
`docs/superpowers/plans/2026-11-27-fase3-p1.md`.

Questo file nasce diviso in tre sezioni, una per card di P1, perché due card lavorano insieme: **ogni card scrive
solo nella sua**. La tabella dei criteri qui sotto le card non la toccano: la riempie chi unisce i tre rami, dagli
esiti scritti nelle sezioni.

## Criteri di P1

| # | Criterio | Esito | Prova |
|---|---|---|---|
| U1 | R-A: «tutto nel tunnel» con WireGuardNT verso un server oltre il gateway | non eseguita | — |
| U2 | R-B: `PUSH_REPLY` da `stdout` prima di `CONNECTED`, DNS messo da noi, su 2.6 e 2.7 | non eseguita | — |
| U3 | R-C: IPv6 bloccato da una sessione WFP dinamica con OpenVPN | non eseguita | — |
| U4 | La VM si accende da uno snapshot e ci si entra via SSH | non eseguita | — |
| U5 | Dalla VM si raggiungono i server del laboratorio | non eseguita | — |
| U6 | Il workspace compila e si prova su `windows-latest`, 0 `ignored` | non eseguita | — |
| U7 | Profili e stato nelle cartelle dell'utente, non leggibili da altri | non eseguita | — |
| U8 | Linux non se ne accorge | non eseguita | — |

Gli esiti ammessi per U1–U3 sono **passa**, **cade**, **non concludente**. «Non eseguita» non è «passa».

**La fase regge?** Non lo sappiamo ancora: la riga la scrive la card P1-A2.

## P1-A1 · La VM Windows 11 e la rete verso il laboratorio (T1, T2)

Card `t_b3cede1b`. Da scrivere.

## P1-A2 · Le tre prove di rischio (T3a, T3b, T3c)

Card `t_5d2dccab`. Da scrivere.

## P1-B · Il workspace compila su Windows (T4a, T5, T4b)

Card `t_4faef896`. Da scrivere.

### Prove che restano di Linux

Da scrivere nel T4a: file, numero di prove, motivo. Base di partenza, misurata il 2026-11-28 a `8aef4f1`:
`cargo test --workspace --exclude galleria-gui` dà 598 passate, 0 fallite, 103 `ignored`.

### Cosa guardare al primo run di `win-check`

Da scrivere nel T4b.

## Per chi viene dopo

Da scrivere.
