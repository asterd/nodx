# NODX-RFC-0004 — Editor & WYSIWYG Contract (Draft)

> Stato: Draft, post-1.0
> Dipende da: NODX-RFC-0001, crate `nodx-cst`
> Scopo: definire il contratto minimo tra sorgente NODX e strumenti di editing strutturale (WYSIWYG, block editor, viewer interattivi).

---

## 1. Motivazione

NODX si presta a editor strutturali grazie all'AST a nodi e agli ID stabili. Senza un contratto formale, però, ogni editor rischia di riscrivere il sorgente in modo non canonico, perdere semantica, o rompere il roundtrip con strumenti CLI/agent.

## 2. Non-goals

- Specificare un editor di riferimento.
- Imporre un'UI o un modello di interazione.
- Garantire roundtrip byte-identico in modalità "structured" (vedi sezione 5).

## 3. Livelli di roundtrip

| Livello | Garanzia | Uso |
|---|---|---|
| Semantic | AST → source canonico, AST identico al re-parse. | Generatori, import, agent mutation. |
| Lossless (CST) | Conserva commenti, spacing, ordine attributi sorgente, stile di fence. | Editor umani, IDE. |

Il livello CST è offerto dal crate `nodx-cst` e dal profilo `editor`.

## 4. Editing modes consigliati

1. `source`: editor testuale con syntax, snippets, diagnostics.
2. `structured`: block editor che manipola AST e rigenera sorgente canonico.
3. `preview`: rendering sicuro HTML/TUI/PDF bridge.
4. `inspect`: AST/NCP/diagnostics/package/security.
5. `a11y`: checklist accessibile con quick fixes.

## 5. Contratto minimo per editor WYSIWYG

| Funzione editor | Requisito NODX |
|---|---|
| Outline | Derivato da heading + section + TOC/NCP. |
| Inspector blocco | Modifica `id`, classi, attrs, profile-specific attrs. |
| Table editor | Mantiene grid coerente, header/scope, caption. |
| Image editor | Richiede `alt` o `decorative=true`, mostra asset package. |
| Style panel | Scrive token/theme/YAML style, non CSS arbitrario non validato. |
| Source roundtrip | Semantic obbligatorio, CST consigliato. |
| Validation panel | Mostra diagnostics con quick fixes. |
| Package view | Mostra manifest, assets, digest, warnings sicurezza. |
| Accessibility panel | Mostra solo problemi azionabili, non rumore tecnico. |

## 6. Primitive CST minime

Il profilo `editor` deve esporre:

- node ranges (offset start/end nel sorgente);
- stable IDs (anche su nodi senza `id` autoriale, via path-based hash);
- local patch (modifica di sotto-albero con preservazione del resto);
- diagnostics mapping (errore → range sorgente).

Tutto il resto (autocomplete, snippets, formatter) è responsabilità dell'editor.

## 7. Roadmap

Promozione a stabile dopo:
- VSCode extension che usa il profilo editor in modalità structured;
- almeno un secondo editor (Neovim plugin o web playground) che fa roundtrip CST;
- fixture conformance per local patch e ID stabili;
- documentazione autoriale "Come scrivere un editor NODX strutturato".
