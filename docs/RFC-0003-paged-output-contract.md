# NODX-RFC-0003 — Paged Output Contract (Draft)

> Stato: Draft, post-1.0
> Dipende da: NODX-RFC-0001
> Scopo: definire un contratto deterministico tra sorgente NODX e renderer paged (PDF, stampa, viewer paged).

---

## 1. Motivazione

La 1.0 di NODX include manual breaks (`::pagebreak`) e dichiarazioni di tema `print`, ma non promette layout PDF byte-identico tra motori diversi. Il Paged Output Contract chiude questa lacuna definendo cosa un renderer paged conforme deve garantire e cosa resta libero.

## 2. Non-goals

- Byte-identical PDF tra motori diversi.
- Motore tipografico interno a NODX.
- Hyphenation, kerning, justification rules custom.

## 3. Livelli di paginazione

| Livello | Meccanismo | Responsabilità |
|---|---|---|
| Manuale | `::pagebreak` | Autore forza nuova pagina. |
| Dichiarativa | `@page`, `page-break-*`, `orphans`, `widows`, `break-inside` | Tema/stile guida il paged renderer. |
| Automatica | Host paged-media contract | Browser/PDF engine calcola pagine reali. |

## 3.1 Inferenza dei page break

Un processore NODX non deve inferire nuovi nodi `pagebreak` nel Canonical AST in
base alla dimensione del foglio. La dimensione effettiva di una pagina dipende da
font disponibili, metriche del motore layout, immagini risolte, margini,
hyphenation, widows/orphans e scaling delle tabelle. Queste informazioni sono
specifiche del renderer paged o dell'host.

Un renderer paged conforme può e deve calcolare automaticamente i confini pagina
durante il layout finale. Questi confini sono output layout, non sorgente
semantico. Se un viewer vuole mostrare una preview paginata, deve etichettarla
come preview host e non come paginazione canonica.

In sintesi:

- `::pagebreak` è l'unico page break autoriale e stabile nel sorgente;
- `@page`, `break-*`, `orphans`, `widows` sono vincoli dichiarativi;
- i page break automatici sono responsabilità del renderer paged e non entrano
  in AST, NCP, signature o conformance parser.

## 4. Requisiti minimi per renderer paged conforme

1. `::pagebreak` produce sempre una nuova pagina, indipendentemente dal contesto.
2. `@page` con `size`, `margin` deve essere rispettato.
3. `page-break-before`, `page-break-after`, `break-inside: avoid` devono essere onorati.
4. `orphans` e `widows` devono accettare almeno valori 1-4.
5. Tabelle larghe devono produrre overflow controllato (scroll in HTML, scaling in PDF) — politica documentata dal renderer.
6. Figure con caption non devono separarsi dalla caption (`break-inside: avoid` implicito).
7. Heading non devono restare orfani in fondo pagina (`break-after: avoid` implicito su heading).
8. Il renderer può dividere paragrafi lunghi, liste e tabelle tra pagine solo nel
   proprio layout output; non deve riscrivere il sorgente NODX con page break
   inferiti.

## 5. Tema `print` standard

Il tema `print` di default deve impostare:

- `@page { size: A4; margin: 22mm; }`
- `h1, h2, h3 { break-after: avoid; }`
- `table, figure { break-inside: avoid; }`
- `orphans: 3; widows: 3;`

Override permessi via tema custom o `::style` con `scope="print"`.

## 6. Canonical AST e paginazione

- La paginazione **non** influisce sul Canonical AST.
- `::pagebreak` è un nodo AST normale.
- Differenze di pagina tra motori diversi **non** rompono NCP né signature.

## 7. Conformance fixture richieste

- `paged/manual-break.nodx` — verifica `::pagebreak`.
- `paged/heading-avoid.nodx` — heading non orfani.
- `paged/figure-caption.nodx` — figure+caption insieme.
- `paged/table-overflow.nodx` — tabella larga, overflow controllato.
- `paged/widows-orphans.nodx` — verifica regole tipografiche minime.

## 8. Roadmap

Questa RFC è draft. Promozione a stabile dopo:
- almeno 2 renderer paged indipendenti che passano le fixture;
- esempio end-to-end in `examples/paged/`;
- documentazione autoriale "Come produrre un PDF professionale da NODX".
