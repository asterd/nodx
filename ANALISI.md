# NODX — Analisi Approfondita e Roadmap verso il Mercato

> Documento di analisi critica e proposta evolutiva  
> Basato su: `NODX-RFC-0001.md` · `README.md` · `IMPLEMENTER_GUIDE.md`  
> Data analisi: 11 maggio 2026

---

## Indice

1. [Valutazione generale](#1-valutazione-generale)
2. [Human-friendliness della sintassi](#2-human-friendliness-della-sintassi)
3. [Proposta di sintassi ridotta (NODX-Lite)](#3-proposta-di-sintassi-ridotta-nodx-lite)
4. [Congruenza degli esempi](#4-congruenza-degli-esempi)
5. [Chiarezza degli approcci](#5-chiarezza-degli-approcci)
6. [Gestione degli stili CSS / NODS](#6-gestione-degli-stili-css--nods)
7. [Evoluzioni già suggerite — valutazione](#7-evoluzioni-già-suggerite--valutazione)
8. [Proposte aggiuntive per market-readiness](#8-proposte-aggiuntive-per-market-readiness)
9. [Roadmap prioritizzata](#9-roadmap-prioritizzata)
10. [Riepilogo delle modifiche raccomandate](#10-riepilogo-delle-modifiche-raccomandate)
11. [Piano evolutivo per una 1.0 forte](#11-piano-evolutivo-per-una-10-forte)
12. [Governance della 1.0 — scope, breaking, metriche, costi](#12-governance-della-10--scope-breaking-metriche-costi)

**Documenti correlati (RFC di supporto):**
- [RFC-0003 — Paged Output Contract](docs/RFC-0003-paged-output-contract.md) (post-1.0)
- [RFC-0004 — Editor & WYSIWYG Contract](docs/RFC-0004-editor-contract.md) (post-1.0)
- [RFC-0005 — Parquet/Arrow Portability](docs/RFC-0005-parquet-arrow-portability.md) (post-1.0)

---

## 1. Valutazione generale

### Cos'è NODX in una frase

NODX è un formato documento UTF-8 strutturato a nodi, pensato per essere deterministico, sicuro, portabile e leggibile da agenti AI — un punto di incontro tra Markdown (leggibilità), XML (struttura), e JSON (serializzazione canonicizzata).

### Punti di forza genuini

**Solidità architetturale.** La RFC è tecnicamente rigorosa. Il modello di sicurezza è ben pensato (fail-closed per default, nessuna esecuzione di script, nessun fetch remoto, limiti di risorse espliciti). Il Canonical AST JSON e la proiezione NCP sono scelte eccellenti per l'era degli agenti AI.

**Scelta del profilo corretta.** Il sistema di profili (`plain`, `core`, `rich`, `style`, `package`, `agent-read`) è una mossa intelligente: permette implementazioni parziali conformi. È meglio di un unico monolite.

**Determinismo.** La scelta di avere un Canonical AST JSON byte-stabile è una garanzia importante che Markdown non ha mai dato. Questo è un vantaggio reale per tooling, firma, e confronto semantico.

**Sicurezza.** Il trattamento del ZIP package, l'audit NODS, i limiti sulle URI, il rifiuto di BOM e U+0000 sono tutti segni di progettazione matura.

### Criticità principali

**Gap tra potenza e accessibilità.** L'RFC è scritta per implementatori, non per autori di documenti. Un writer tecnico che la legge capirà la struttura, ma non avrà mai la sensazione di "voglio scrivere in questo formato oggi".

**Verbosità del front matter.** Per un documento semplice si devono dichiarare profili, schema, type — una cerimonia che Markdown non richiede.

**La sintassi `:::` è tecnica ma non intuitiva.** Funziona, è non ambigua, ma ha un feeling da "linguaggio intermedio" più che da "formato autoriale".

**Nessun percorso di adozione graduale documentato.** Un utente Markdown non sa dove iniziare. Non c'è una risposta alla domanda: "Cosa posso fare con NODX che non posso fare con Markdown?"

**Mancanza di una proposition narrativa chiara.** Il README descrive cosa contiene il repo, non perché qualcuno dovrebbe adottare NODX.

---

## 2. Human-friendliness della sintassi

### Analisi della sintassi attuale

#### Front matter — verbosità necessaria ma gestibile

```yaml
---
schema: nodx/1.0
type: document
title: Hello NODX
profiles:
  requires:
    - core
    - rich
  optional:
    - style
    - agent-read
---
```

**Problema:** Per un documento base occorrono 10 righe di boilerplate. La dichiarazione dei profili è utile per i processori ma oscura per un autore.

**Soluzione proposta:** il profilo `plain` dovrebbe essere implicito quando manca front matter. Il profilo `core` dovrebbe essere assunto dal parser quando il documento contiene heading/liste senza dichiarazione esplicita. Solo `rich` e `style` dovrebbero richiedere dichiarazione esplicita.

#### Blocchi delimitati — il punto più critico

La sintassi `:::nome {attrs}` è funzionale ma ha due problemi:

1. **Il nome del nodo è l'unico differenziatore semantico** — `:::note`, `:::section`, `:::figure` devono essere ricordati. Non c'è segnale visivo immediato.
2. **La chiusura `:::` senza nome** è ambigua alla lettura rapida.

**Confronto di leggibilità:**

Markdown (familiare, limitato):
```markdown
> Nota importante
```

NODX attuale (potente, verboso):
```
:::note {#risk .warning fallback="children"}
Nota importante
:::
```

NODX proposto (equilibrio):
```
::note {#risk .warning}
Nota importante
::
```

#### Attributi — quasi perfetti

La sintassi `{#id .class key="value"}` è buona — deriva da Pandoc e è già conosciuta da molti scrittori tecnici. Da tenere com'è.

#### Inline — ottimo

`**strong**`, `*em*`, `` `code` ``, `[link](url)` — compatibile con Markdown, nessun problema. Le aggiunte (`==mark==`, `~sub~`, `^sup^`, `$$math$$`) sono naturali e non conflittuali.

#### Variabili `{{vars.name}}` — buono ma namespace rigido

Il namespace `vars.` è obbligatorio. `{{vars.reviewer}}` vs `{{reviewer}}` — la seconda è più leggibile. Il namespace dovrebbe essere opzionale quando non ambiguo.

---

## 3. Proposta di sintassi ridotta (NODX-Lite)

Questa proposta deve essere considerata parte della 1.0 di prodotto, anche se richiede di aggiornare la bozza tecnica corrente. La regola chiave è: ogni forma lite deve avere una mappatura deterministica verso lo stesso Semantic AST delle forme estese. La sintassi sorgente può diventare più amichevole; Canonical AST, NCP, sicurezza e output devono restare rigorosi.

### 3.1 Front matter opzionale ridotto

```yaml
---
title: Il mio documento
---
```

Il parser assume implicitamente:
- `schema: nodx/1.0`
- `type: document`
- `profiles.requires: [core]` — escalato automaticamente a `rich` se il documento usa tabelle/figure/toc

Questo elimina 7 righe su 10 per il caso comune.

### 3.2 Blocchi con doppio colon (forma corta) — regola semplificata

Proposta finale: `::nome` come **unica** forma corta valida ovunque (foglia o annidato), con chiusura `::` semplice oppure `::nome` per chiusure etichettate. La forma a tre coloni `:::` resta valida come legacy/escape ma non è obbligatoria per i casi annidati.

```
::note {.warning}
Questa è una nota di avviso.
::

::figure {#pipeline}
::image {src="assets/pipeline.png" alt="Pipeline"}
::
::caption
Figura 1: il pipeline di elaborazione.
::
::
```

**Regole:**

1. `::nome` apre un blocco; `::` chiude il blocco più interno aperto.
2. `::nome` può anche essere usato come chiusura etichettata: deve corrispondere al nome dell'apertura più recente non ancora chiusa.
3. Il numero di coloni di apertura e chiusura deve coincidere. Per casi rari di conflitto sintattico (es. contenuto che inizia con `::`), l'autore può usare `:::` o più coloni come escape.
4. Non c'è distinzione tra "blocco foglia" e "blocco annidato": l'autore non deve decidere a priori se userà figli.

**Cosa è stato rimosso rispetto alla bozza precedente:** la regola "`::` per foglie, `:::` obbligatorio per annidati" è stata eliminata perché generava ambiguità autoriale (cambio di coloni se cambia idea sui figli) e non portava beneficio reale.

### 3.3 Profili dichiarati inline con shorthand — RIMANDATO POST-1.0

**Decisione:** lo shorthand `profiles: core+rich` non entra nella 1.0.

**Motivazione:** con il front matter implicito (R1), l'utente medio non scrive mai `profiles:` esplicitamente. Il problema della verbosità si risolve da solo. Introdurre un mini-dialetto YAML custom (anche se ben recintato) aggiunge superficie di parsing senza beneficio per il caso comune. Resta valutabile in 1.x se emergerà come pain point reale.

**Forma 1.0 supportata:**
```yaml
profiles:
  requires:
    - core
    - rich
```

### 3.4 Variabili senza namespace obbligatorio

```
Revisore: {{reviewer}}        # equivale a {{vars.reviewer}}
Build: {{meta.title}}         # namespace esplicito quando non è vars
```

Regola: se il nome non contiene `.`, il parser assume namespace `vars`. Se contiene `.`, è un accesso qualificato (`meta.title`, ecc.).

### 3.5 Attributi light — solo `#id` finale

Per il caso più frequente (ID stabile su heading), ammettere la forma senza graffe **solo** per un singolo `#id` a fine riga:

```
# Titolo #intro
```

Invece di:
```
# Titolo {#intro}
```

**Regole strette:**

1. La forma light è valida **solo** alla fine della riga di un heading, dopo uno spazio.
2. Token: `#` seguito da identificatore ASCII (`[A-Za-z][A-Za-z0-9_-]*`).
3. Non è ammessa la forma `# Titolo .class` light: la classe richiede sempre graffe `{.class}`. Motivo: heading che terminano con frasi tipo "versione 3.0.lead" diventerebbero ambigui.
4. Per heading che terminano con `#` letterale, l'autore può usare l'escape `\#intro` o la forma piena `{#intro}`.
5. La forma con graffe `{...}` resta canonica e necessaria per: attributi multipli, classi, chiave-valore.

**Cosa è stato rimosso rispetto alla bozza precedente:** la forma `# Titolo .lead` come light per classi. Restringere ai soli ID riduce ambiguità senza perdere il 90% del beneficio (l'ID è il caso davvero frequente per link interni, TOC, NCP).

### 3.6 TOC implicita

```
::toc
::
```

invece di:

```
:::toc {#contents role="primary" depth="2" title="Contents"}
:::
```

Il parser genera automaticamente `id`, `role="primary"`, `depth=6`, `title` deterministico dalla lingua del documento.

### 3.7 Tabella: separatore colonne alternativo

La RFC richiede `| - |` per separare header. Ammettere anche `|---|`:

```
| Colonna A | Colonna B |
|-----------|-----------|
| valore    | valore    |
```

Entrambe le forme valide, AST identico.

### 3.8 Link con attributi e titolo

Markdown supporta gia il concetto di label visibile separata dal target. NODX dovrebbe estenderlo in modo naturale con attributi post-link:

```
[documentazione](docs/guide.nodx){title="Apri la guida" rel="help"}
[scarica il report](assets/report.pdf){download="report-q1.pdf"}
```

Regole:
- `[label](target)` resta la forma base.
- `[label](target){attrs}` produce un inline `link` con `label`, `target` e `attrs`.
- `title` e un attributo accessibile/rendering, non sostituisce la label visibile.
- Gli stessi controlli URL di sicurezza restano obbligatori.

Questa forma evita workaround con span annidati e rende NODX piu vicino alle aspettative di chi arriva da Markdown/HTML.

### 3.9 TOC con titoli di riga sovrascrivibili

Il `toc` deve permettere di controllare sia il titolo della navigazione sia il testo delle singole righe indice.

Forma attuale utile:
```
::toc {title="Indice" depth="2"}
::
```

Proposta per sovrascrivere una voce del TOC direttamente sull'heading:
```
# Capitolo 1: Architettura del sistema {#arch toc="Architettura"}
## Dettaglio implementativo molto lungo {#impl toc="Implementazione"}
```

Regole:
- il testo dell'heading resta il titolo visibile nel documento;
- `toc="..."` diventa il titolo usato nelle entry generate da `toc`;
- se `toc` manca, si usa il plain text dell'heading;
- `toc-hidden="true"` esclude un heading dalla navigazione generata;
- `toc-level="2"` permette, solo se necessario, di normalizzare una voce nella navigazione senza cambiare il livello visivo dell'heading.

Per casi editoriali complessi serve anche una forma manuale:

```
::toc {title="Percorso consigliato" mode="manual"}
- [Introduzione breve](#intro)
- [API essenziali](#api)
- [Appendice completa](#appendix)
::
```

Il modo manuale e utile per guide, landing docs, report e documenti molto lunghi dove l'indice deve essere narrativo, non una semplice copia della gerarchia.

---

## 4. Congruenza degli esempi

### Esempio nel README

L'esempio nel README è **congruente con la RFC** ma manca di progressione. Mostra subito TOC, profili, tabelle e note — troppo per un primo contatto.

**Problema:** Un lettore che vede quel blocco come primo esempio non riesce a identificare cosa è "base" e cosa è "avanzato". È come mostrare un programma Rust con lifetime, traits e generics come primo esempio di Rust.

**Proposta — tre livelli progressivi:**

```
# Esempio minimo
---
title: Il mio primo documento NODX
---

# Ciao NODX

Questo è un paragrafo con testo **in grassetto** e un [link](https://example.com).
```

```
# Esempio intermedio (con struttura)
---
title: Report Q1
profiles:
  requires: [core, rich]
---

# Report Q1 {#q1}

Sintesi del trimestre.

| Metrica | Valore |
| - | - |
| Ricavi | 120K |
| Costi | 80K |
```

```
# Esempio avanzato (componenti, stile, TOC)
...il blocco attuale del README...
```

### Esempi nel repo

Da quanto visibile nella struttura del repo gli esempi coprono bene i casi d'uso. I problemi sono:

1. **Non c'è un indice visivo degli esempi** — un visitatore non sa qual esempio guardare prima.
2. **Gli esempi di stampa e i18n non appaiono nel README** — sembrano features di serie B quando sono invece molto rilevanti.
3. **`layout-fonts.nodx`** è citato solo di sfuggita; la capacità di gestire layout/font è un differenziatore importante.

---

## 5. Chiarezza degli approcci

### Cosa funziona bene

- Il modello di profili (sezione 2.3) — tabella chiara ✓
- Il Canonical AST (sezione 9) — definito con precisione ✓
- La politica URL/asset (sezione 19) — rigorosa e ben articolata ✓
- Il processing model (sezione 4) con 11 passi ordinati ✓

### Cosa non è chiaro

**La relazione tra NODS e CSS.** La sezione 18 descrive NODS come "safe subset for literal style blocks" ma non spiega abbastanza come si relaziona al CSS normale. Un autore che conosce CSS deve capire cosa può e non può usare — la lista dei "forbidden constructs" è utile ma serve anche una lista positiva dei selettori e proprietà ammessi.

**Quando usare `:::section` vs `:::note` vs heading semplice.** La distinzione semantica tra questi costrutti non è intuitiva. Serve una tabella "quando usare cosa".

**Il percorso package.** La gestione ZIP package (sezione 20) è tecnicamente precisa ma visivamente distante dall'autore. Non è chiaro quando un documento "normale" diventa packaged e come si crea praticamente il package.

**La firma digitale** è menzionata come "reserved" in vari punti ma non è chiaro se è disponibile oggi o no. La presenza di `signature:` nel manifest schema genera confusione.

**Il modello di fallback dei componenti custom** è un'idea brillante (`fallback="children"`) ma non viene spiegata con abbastanza esempi concreti di cosa succede su un renderer che non conosce il componente.

---

## 6. Gestione degli stili CSS / NODS

### Situazione attuale

La sezione 18 definisce NODS come subset sicuro di CSS all'interno di blocchi `:::style`. L'approccio è corretto per sicurezza, ma ha lacune pratiche:

1. Solo CSS puro inline nel documento — nessun file separato o sezione globale distinta.
2. Nessun tema predefinito — ogni documento parte da zero.
3. Nessuna variabile CSS nativa documentata (custom properties `--var` sono "allowed" ma non evidenziate).
4. Nessun concetto esplicito di "stile di stampa" vs "stile web".

### Proposte concrete per il sistema stili

#### 6.1 Doppia modalità di stile (CSS puro + YAML)

Il parser dovrebbe accettare entrambe le forme nel blocco `:::style`:

**Forma CSS (già esistente):**
```css
:::style
h1 { color: #0f766e; font-size: 24pt; }
@media print { p { color: black; } }
:::
```

**Forma YAML (proposta):**
```yaml
:::style {format="yaml"}
h1:
  color: "#0f766e"
  font-size: 24pt
  font-family: Georgia, serif
table:
  border-collapse: collapse
  width: 100%
print:
  p:
    color: black
:::
```

La forma YAML è più validabile staticamente, evita ambiguità di parsing CSS e consente tooling migliore. Entrambe le forme producono lo stesso Canonical AST.

**Paletto pseudo-keys YAML (vincolo 1.0):**

Per evitare che YAML style diventi un linguaggio CSS parallelo, le chiavi top-level che non sono selettori CSS devono appartenere a un set chiuso e documentato di "at-rule pseudo-keys":

| Pseudo-key | Mappa a CSS |
|---|---|
| `print` | `@media print { ... }` |
| `screen` | `@media screen { ... }` |
| `dark` | `@media (prefers-color-scheme: dark) { ... }` |
| `page` | `@page { ... }` |

Qualsiasi altra chiave top-level deve essere un selettore CSS valido (es. `h1`, `.warning`, `table th`). Pseudo-keys non riconosciute sono **errore di validazione**, non vengono ignorate silenziosamente. Questo paletto è necessario per evitare che il YAML style si trasformi in un sotto-linguaggio CSS proprietario.

#### 6.2 Temi predefiniti con `theme:` in front matter

```yaml
---
title: Report Annuale
theme: print          # oppure: web, base, none
---
```

Temi predefiniti inclusi nella distribuzione standard:

| Nome tema | Descrizione |
|---|---|
| `base` | Reset + tipografia minimal, nessun colore |
| `web` | Stile web responsive con tipografia moderna |
| `print` | Ottimizzato per PDF/stampa, A4, margini corretti |
| `presentation` | Per slide, font grande, contrasto alto |
| `plain` | Solo plain text, CSS minimale |

#### 6.3 File `.nodt` (NODX Theme)

Un file tema separato con estensione `.nodt` referenziato nel front matter:

```yaml
---
title: Documento Aziendale
theme: themes/corporate.nodt
---
```

Struttura di un file `.nodt`:
```yaml
schema: nodx-theme/1.0
name: corporate
extends: web              # override di un tema base
vars:
  primary: "#003087"
  secondary: "#c41e3a"
styles: |
  h1 { color: var(--primary); }
  .warning { border-color: var(--secondary); }
```

#### 6.4 Override inline sopra tema base

```
:::style {scope="document"}
h1 { font-size: 28pt; }  /* override del tema web */
:::
```

L'attributo `scope="document"` indica che questo stile sovrascrive il tema, con priorità documentata nella cascade.

#### 6.5 CSS custom properties come bridge

I temi predefiniti dovrebbero esporre custom properties standard:

```css
:root {
  --nodx-color-primary: #0f766e;
  --nodx-color-accent: #b91c1c;
  --nodx-font-body: Georgia, serif;
  --nodx-font-mono: 'JetBrains Mono', monospace;
  --nodx-font-size-base: 11pt;
  --nodx-page-margin: 22mm;
  --nodx-line-height: 1.6;
}
```

Gli autori sovrascrivono solo le variabili che vogliono cambiare.

---

## 7. Evoluzioni già suggerite — valutazione

### 1. CSS gestito come CSS puro + YAML con parser duale — Priorità ALTA ✓

**Valutazione:** Ottima idea. Il parser duale è implementabile come pre-processore YAML→CSS interno, senza cambiare il Canonical AST.

**Rischio:** La specifica YAML del CSS non deve diventare un sotto-linguaggio CSS proprietario. Deve mappare 1:1 sulle proprietà CSS ammesse da NODS.

### 2. Temi standard e file `.nodt` — Priorità ALTA ✓

**Valutazione:** Fondamentale per l'adozione. Un utente che vede "scrivi il documento, aggiungi `theme: print` e hai un PDF professionale" converte immediatamente.

**Note implementative:** I temi non devono essere embedded nell'RFC ma in un registro separato di risorse della distribuzione. Il formato `.nodt` merita una propria mini-spec (RFC-0002).

### 3. Attributi light — Priorità MEDIA ✓

**Valutazione:** Utile per ridurre verbosità sugli heading. Attenzione: la forma `# Titolo #intro` è ambigua se si vuole un heading che finisce con un `#` letterale. Il parser dovrebbe richiedere che la forma light sia solo alla fine della riga.

### 4. Sezione esempi dedicata agli stili — Priorità ALTA ✓

**Valutazione:** Necessaria. Una galleria di esempi visivi (con rendering HTML associato) vale più di 100 righe di spec.

### 5. Quickstart e Cheat Sheet nel README — Priorità MOLTO ALTA ✓

**Valutazione:** Questa è la priorità #1 per l'adozione esterna. Servono:
- Una sezione "In 5 minuti" con i 10 costrutti fondamentali
- Un cheat sheet scaricabile (PDF/SVG)
- Una risposta chiara a "perché NODX invece di Markdown?"

### 6. Migliorare gli esempi live — Priorità ALTA ✓

**Valutazione:** Il playground web (`apps/web/playground.html`) è già presente ma non è menzionato abbastanza prominentemente. Dovrebbe essere il secondo link nella sezione introduttiva, subito dopo la RFC.

### 7. Rendering Python e plugin VSCode — Priorità CRITICA ✓✓

**Valutazione:** Questi due elementi sono *prerequisiti* per l'adozione, non nice-to-have.

**Python:** La maggior parte del workflow di generazione documenti nel mondo enterprise usa Python. Un `pip install nodx-py` con API semplice è la via più rapida per l'adozione enterprise.

**VSCode:** È l'editor più usato. Senza syntax highlighting e preview in VSCode, NODX è praticamente invisibile per la maggior parte degli sviluppatori.

---

## 8. Proposte aggiuntive per market-readiness

### 8.1 Proposition narrativa chiara

Il README deve rispondere esplicitamente a: **"Perché NODX invece di Markdown?"**

Risposta suggerita (da inserire nelle prime righe):

> NODX fa ciò che Markdown non ha mai potuto fare: struttura semantica vera, output deterministico per gli agenti AI, sicurezza by default, temi di stampa professionali, e portabilità del documento come pacchetto ZIP con assets — tutto senza un server, senza JavaScript, senza plugin. Se hai bisogno di documenti che devono essere letti, validati, firmati, e consumati da macchine allo stesso modo in cui li leggono le persone, NODX è la risposta.

### 8.2 Livelli di adozione documentati

**Livello 1 — Autore:** scrive `.nodx` come se fosse Markdown arricchito, usa `theme: web`, genera HTML. Non dichiara profili, non sa cosa sia il Canonical AST.

**Livello 2 — Integratore:** costruisce pipeline che producono PDF/DOCX da `.nodx`, usa NCP per fare ricerca semantica, integra nel proprio CMS.

**Livello 3 — Implementatore:** scrive un parser NODX in un nuovo linguaggio, contribuisce conformance test, implementa un profilo riservato.

Ogni livello ha la propria documentazione di ingresso.

### 8.3 Comparison table con formati esistenti

| Caratteristica | Markdown | AsciiDoc | LaTeX | NODX |
|---|---|---|---|---|
| Leggibile come testo | ✓ | ✓ | ✗ | ✓ |
| AST canonico stabile | ✗ | ~ | ✗ | ✓ |
| Sicuro per input non fidati | ✗ | ✗ | ✗ | ✓ |
| Tema stampa built-in | ✗ | ~ | ✓ | ✓ |
| Pacchetto ZIP con assets | ✗ | ✗ | ✗ | ✓ |
| Proiezione per agenti AI | ✗ | ✗ | ✗ | ✓ |
| Profili di conformità | ✗ | ✗ | ✗ | ✓ |
| Plugin VSCode maturo | ✓✓ | ✓ | ✓ | work in progress |

### 8.4 NODX-RFC-0002: Theme Format

La gestione dei temi merita una mini-RFC separata. Definisce:
- Schema `nodx-theme/1.0`
- Estensione `.nodt`
- Campo `extends` per ereditarietà dei temi
- Registro dei temi standard (`base`, `web`, `print`, `presentation`)
- Custom properties standard (`--nodx-*`)
- Regole di override e specificity

### 8.5 Wizard di migrazione da Markdown

```bash
nodx import markdown input.md --report conversion.json > output.nodx
```

Abbassa enormemente la barriera per chi ha già contenuto in Markdown.

### 8.6 JSON Schema per front matter

Pubblicare un JSON Schema validabile con editor moderni — permette autocomplete nel front matter senza implementare il parser completo.

### 8.7 Error code registry machine-readable

L'error code registry (sezione 23.1) dovrebbe essere esportato come `spec/error-registry.json` usabile da linter, editor, e CI.

### 8.8 Test di accessibilità nell'output HTML

Il renderer HTML dovrebbe avere test automatici per:
- presenza di `lang` sul `<html>`
- `alt` su tutte le immagini non decorative
- heading order senza salti
- tabelle con `<th scope="col">`

### 8.9 Modalità `--watch` nella CLI

```bash
nodx html examples/report.nodx --watch --output target/report.html
```

Fondamentale per il workflow editoriale. Prerequisito pratico prima del plugin VSCode.

### 8.10 Versioning dei temi separato dalla versione del formato

I temi possono evolvere più rapidamente del formato. Separare il versioning evita di bumppare la RFC per cambiare un tema.

---

## 9. Roadmap prioritizzata

### Fase 1 — Accessibilità (0–3 mesi)

1. **README refactor** con proposition chiara, quickstart in 5 minuti, link al playground prominente
2. **Cheat sheet** (PDF/SVG scaricabile) con i 20 costrutti fondamentali
3. **VSCode extension MVP**: syntax highlighting + snippets + `nodx diagnostics` integration
4. **Front matter implicito**: parser che assume `core` quando manca front matter
5. **`theme:` nel front matter** con `web` e `print` predefiniti

### Fase 2 — Ecosistema (3–6 mesi)

1. **`pip install nodx-py`** con API Python semplice: `parse()`, `render_html()`, `render_pdf()`
2. **NODX-RFC-0002**: Theme format `.nodt` con ereditarietà
3. **Wizard Markdown→NODX** con report di conversione
4. **Sezione esempi stili** nel sito/docs con galleria visiva
5. **JSON Schema front matter** pubblicato
6. **Modalità `--watch`** nella CLI
7. **YAML style blocks** come alternativa a CSS raw

### Fase 3 — Maturità (6–12 mesi)

1. **Sito web dedicato** (nodx.dev o simile) con docs strutturate per livello
2. **Comparison page** con Markdown/AsciiDoc/LaTeX
3. **Temi aggiuntivi**: `presentation`, `academic`, `technical-report`
4. **Plugin Neovim/Emacs**
5. **GitHub Actions action** per CI/CD di documenti NODX
6. **NODX-RFC-0003**: Presentation profile stabile
7. **Registry online di temi e componenti custom**

---

## 10. Riepilogo delle modifiche raccomandate

### Modifiche alla RFC (RFC-0001 o errata)

| # | Modifica | Impatto |
|---|---|---|
| R1 | Front matter implicito con inferenza profilo | Alta accessibilità |
| R2 | Forma lite `::nome` (unica regola, valida ovunque) | Riduce verbosità senza ambiguità |
| ~~R3~~ | ~~`profiles: core+rich` shorthand~~ — **rimandato post-1.0** | R1 risolve il problema |
| R4 | Variabili senza namespace `vars.` obbligatorio | Leggibilità |
| R5 | Attributi light **solo `#id` finale** sugli heading | Leggibilità senza ambiguità su classi |
| R6 | Separatore tabella `\|---\|` alternativo | Compatibilità Markdown |
| R7 | Link con `{attrs}` e title shorthand | Accessibilità e parità con Markdown/HTML |
| R8 | TOC con row override, `toc-hidden`, mode manuale | Navigazione professionale |
| R9 | `theme:` e design tokens standard | Output bello senza CSS manuale |
| R10 | YAML style blocks con paletto pseudo-keys (`print`, `screen`, `dark`, `page`) | Authoring e generazione agent-friendly |
| R11 | Layout/font/color profile minimo | Differenziazione reale da Markdown |
| R12 | i18n/RTL/accessibility come release gate | Adozione enterprise/globale |
| R13 | Table schema profile + CSV/TSV (Parquet/Arrow → RFC-0005) | Sostituto serio di CSV documentato |

### Nuovi documenti/specifiche

| # | Documento | Priorità |
|---|---|---|
| D1 | NODX-RFC-0002: Theme Format `.nodt` | CRITICA |
| D2 | Quickstart Guide (narrativa, non RFC) | CRITICA |
| D3 | Cheat Sheet visuale | ALTA |
| D4 | JSON Schema front matter | ALTA |
| D5 | Error Registry JSON machine-readable | MEDIA |
| D6 | Comparison document vs altri formati | ALTA |

### Nuovo tooling

| # | Tool | Priorità |
|---|---|---|
| T1 | VSCode extension (syntax + snippets + preview) | CRITICA |
| T2 | `nodx-py` Python package | CRITICA |
| T3 | Tema `print` e `web` nella distribuzione | ALTA |
| T4 | Markdown→NODX converter | ALTA |
| T5 | CLI `--watch` mode | ALTA |
| T6 | GitHub Actions action | MEDIA |
| T7 | YAML-format style blocks | MEDIA |
| T8 | `nodx package create` | ALTA |
| T9 | Playground con esempi progressivi | ALTA |

### Modifiche al README

| # | Modifica |
|---|---|
| M1 | Aggiungere proposition narrativa (perché NODX?) nelle prime 5 righe |
| M2 | Sezione "In 5 minuti" come primo contenuto dopo l'abstract |
| M3 | Link al playground come secondo link (subito dopo la RFC) |
| M4 | Progressione degli esempi: minimal → intermedio → avanzato |
| M5 | Comparison table con Markdown/AsciiDoc/LaTeX |
| M6 | Badge CI con stato del conformance runner |

---

## Conclusione

NODX è un formato tecnicamente solido, con una filosofia di design coerente e ben fondata. Ha tutti gli ingredienti giusti per diventare uno standard de facto per i documenti strutturati nell'era degli agenti AI: determinismo, sicurezza, portabilità, proiezione semantica.

Il gap principale non è tecnico — è comunicativo e di ecosistema. Il formato ha bisogno di scendere dal piano della specifica e atterrare sul piano dell'esperienza dell'autore. Questo significa:

1. **Documentazione narrativa** accanto alla RFC tecnica
2. **Tooling di base immediatamente funzionante** (VSCode, Python, temi)
3. **Percorso di adozione graduale** che parte da "è come Markdown ma meglio"

Con queste evoluzioni — la metà delle quali sono documentazione e tooling, non modifiche al formato — NODX ha tutti i presupposti per essere adottato seriamente.

---

## 11. Piano evolutivo per una 1.0 forte

### Tesi aggiornata

Se l'obiettivo e pubblicare NODX come alternativa reale a Markdown, non basta congelare la bozza corrente. La 1.0 deve essere il punto in cui il formato e gia abbastanza semplice da imparare in un pomeriggio, ma abbastanza potente da sostituire Markdown, AsciiDoc e molti usi leggeri di HTML/PDF authoring.

La stabilita resta importante, ma va applicata alla release pubblica finale, non alla bozza interna. Prima della pubblicazione conviene cambiare cio che rende il formato piu adottabile, purche ogni aggiunta abbia:

1. grammatica deterministica;
2. mapping canonico unico verso Semantic AST;
3. test di conformance;
4. comportamento fail-closed per sicurezza;
5. documentazione autoriale chiara.

### Cosa diventa necessario, non opzionale

| Area | Decisione 1.0 | Perche e importante |
|---|---|---|
| Sintassi Lite (R2 semplificata) | Includere nella 1.0 | Senza shorthand, NODX sembra un IR tecnico invece di un formato autoriale. |
| Variabili rapide | Includere `{{name}}` come alias di `{{vars.name}}` | Riduce attrito nei template e nei report generati. |
| Attributi light (solo `#id`) | Includere su heading | Gli ID sono centrali per link, TOC, NCP e agenti; devono essere facili. |
| Link con attributi | Includere `[label](url){attrs}` | Serve per title, download, rel, target controllato, accessibilita. |
| TOC controllabile | Includere titolo navigazione, row title override e manual mode | Un indice generato solo dai titoli lunghi non basta per documenti reali. |
| CSS/NODS YAML | Includere come authoring syntax (con paletto at-rule pseudo-keys) | E piu validabile, piu editor-friendly e piu facile per agenti/generatori. |
| Temi standard | Includere `theme:` e temi `base`, `web`, `print` | Il primo output deve essere bello senza CSS scritto a mano. |
| Layout/font/color system | Includere come parte dello style profile | Layout, font e colori sono differenzianti, non extra. |
| i18n/RTL | Trattarli come standard release gate | Documenti globali e enterprise devono funzionare da subito. |
| ZIP autoconsistente | Trattarlo come esperienza autore, non solo reader | Deve essere facile creare un `.nodx` con assets locali e manifest. |
| Table schema + CSV/TSV | Includere nella 1.0 | Sostituto serio di CSV documentato. |

### Cosa NON entra nella 1.0 (rimandato a spec dedicate)

| Area | Decisione | Spec di riferimento |
|---|---|---|
| Profili shorthand (`core+rich`) | Rimandato. Front matter implicito risolve il problema per il caso comune. | Riconsiderare in 1.x se emerge come pain point. |
| Paged Output Contract | Rimandato a RFC dedicata. La 1.0 supporta `::pagebreak` e tema `print`, ma non garantisce byte-identical PDF tra motori. | [RFC-0003](docs/RFC-0003-paged-output-contract.md) |
| Editor/WYSIWYG/CST contract | Rimandato a RFC dedicata. La 1.0 espone primitive CST minime, il contratto editor formale arriva dopo. | [RFC-0004](docs/RFC-0004-editor-contract.md) |
| Parquet/Arrow portability | Rimandato a RFC dedicata. La 1.0 fa schema tabellare + CSV/TSV; Parquet/Arrow sono export tipizzati post-1.0. | [RFC-0005](docs/RFC-0005-parquet-arrow-portability.md) |
| Firma digitale | Resta `reserved`. Profilo `signature` portabile in 1.x. | — |
| Presentation profile completo | Tema `presentation` base nella 1.0, profilo slide/deck completo post-1.0. | RFC-0006 futura. |
| Registry online temi | Solo `.nodt` locale nella 1.0. | Post-1.0. |

### Vincoli che rendono sicure le scorciatoie

Le scorciatoie sono un problema solo se sono ambigue o se producono AST diversi a seconda dell'implementatore. Con vincoli precisi diventano un vantaggio netto.

**Variabili rapide**

```
{{reviewer}}        # canonical: {{vars.reviewer}}
{{meta.title}}      # namespace esplicito
{{env.BUILD_ID}}    # vietato o host-policy, mai implicito
```

Regola: un nome senza punto e sempre `vars.<name>`. Un nome con punto e qualificato. Namespace riservati (`meta`, `vars`, `doc`, `package`) devono essere documentati. Namespace dinamici o host-specific vanno vietati nella 1.0 portabile.

**Attributi light**

```
# Titolo #intro
# Titolo .lead
# Titolo #intro .lead
::note #risk .warning
```

Regola: attributi light sono validi solo a fine opener/heading, dopo uno spazio, con token `#id` e `.class` ASCII validi. Per `key="value"` resta obbligatoria la forma `{...}`. Se un autore vuole un `#id` letterale alla fine di un titolo, lo escapa: `\#intro`.

**Profili compatti**

```
profiles: core+rich+style
```

Meglio di `profiles: [core, rich]` se si vuole evitare di importare YAML flow-style. E una stringa scalare compatta, non una lista YAML. Canonical AST la normalizza in `profiles.requires`.

**Blocchi lite**

```
::note #risk .warning
Contenuto.
::
```

Regola: `::name` e valido solo a inizio riga, `name` deve iniziare con lettera ASCII, la chiusura deve avere lo stesso numero di coloni. Per annidamenti complessi resta permesso usare tre o piu coloni, ma non deve essere obbligatorio nel caso comune.

### TOC e navigazione

Il TOC deve diventare una feature forte, non solo un blocco tecnico.

**Titolo del blocco indice**

```
::toc {title="Indice" role="primary" depth="2"}
::
```

`title` e la label visibile/accessibile della navigazione.

**Titolo della singola riga indice**

```
# Capitolo 1: Architettura del sistema {#arch toc="Architettura"}
## Come funziona il parser nei casi limite {#parser toc="Parser"}
```

`toc` sull'heading sovrascrive la voce generata. Il titolo completo resta nel corpo del documento.

**Esclusione dalla TOC**

```
### Nota tecnica interna {#internal toc-hidden="true"}
```

**TOC manuale**

```
::toc {title="Percorso consigliato" mode="manual"}
- [Panoramica](#overview)
- [Esempio minimo](#minimal)
- [API completa](#api)
::
```

Il modo manuale deve accettare una lista di link sicuri e produrre lo stesso tipo di navigation entries dell'indice automatico. Questo rende possibile un indice editoriale, non solo strutturale.

### Link

La sintassi base Markdown resta:

```
[label](target)
```

La 1.0 dovrebbe aggiungere:

```
[label](target "title")
[label](target){title="title" rel="help"}
[label](target){download="file.pdf"}
```

Regole:
- la forma `"title"` e zucchero sintattico per `{title="title"}`;
- `{attrs}` e la forma completa;
- `target`, `rel`, `download`, `title` devono essere sottoposti a policy/escaping;
- link remoti restano host-policy se il profilo vuole massima sicurezza;
- link a package assets e fragment ID sono il caso ottimizzato.

### CSS, YAML style e temi

La proposta CSS/YAML non va indebolita. E uno dei punti piu forti per adozione enterprise e agent-generated documents.

**Due forme equivalenti**

```
::style
h1 { color: var(--nodx-color-primary); }
::
```

```
::style {format="yaml"}
h1:
  color: var(--nodx-color-primary)
  font-size: 24pt
print:
  body:
    margin: 20mm
::
```

Regola: YAML style e authoring syntax. Il Canonical AST puo conservare il source normalizzato e una rappresentazione style object deterministica. Il renderer puo emettere NODS/CSS sanitizzato. Non serve supportare tutto CSS: serve supportare un subset serio, documentato e sufficiente per documenti professionali.

**Temi standard 1.0**

```
---
title: Report Q1
theme: print
---
```

Temi minimi obbligatori:

| Tema | Obiettivo |
|---|---|
| `none` | Nessuno stile oltre al semantic HTML minimo. |
| `base` | Tipografia pulita, leggibile, neutra. |
| `web` | Layout responsive per browser/app. |
| `print` | A4/Letter, margini, page break, tabelle e figure stampabili. |
| `presentation` | Base per slide/deck, anche se export avanzato resta successivo. |

**Design tokens**

La 1.0 deve definire token standard:

```
--nodx-color-text
--nodx-color-muted
--nodx-color-primary
--nodx-color-accent
--nodx-font-body
--nodx-font-heading
--nodx-font-mono
--nodx-page-margin
--nodx-line-height
--nodx-block-gap
```

I temi possono evolvere, ma i token base devono essere stabili.

### Layout, font, colori

NODX non deve competere con Markdown solo sulla struttura: deve produrre documenti belli e controllabili senza scendere in HTML libero.

Elementi da includere:

1. blocchi layout sicuri: `columns`, `grid`, `row`, `card`, `callout`;
2. attributi layout limitati: `width`, `align`, `gap`, `span`;
3. font stack dichiarabili via tema, non via fetch remoto automatico;
4. palette colori con token, non colori hardcoded ovunque;
5. print controls: `pagebreak`, `page`, `margin`, `size`;
6. regole responsive sicure nel tema, non script.

Questa e la differenza tra "Markdown con AST" e "document source format moderno".

### i18n, RTL e accessibilita

i18n e RTL devono essere release gates:

| Requisito | Decisione |
|---|---|
| `language` e `dir` | Metadata standard, default sicuri, override per span/blocchi. |
| Mixed scripts | Fixture obbligatorie per CJK, arabo RTL, testo misto. |
| TOC e headings | Plain text extraction Unicode-safe. |
| Font fallback | Temi con stack sensati per Latin/CJK/RTL. |
| PDF/print | Nessuna assunzione LTR hardcoded. |
| Accessibility | `alt`, heading order, table headers, nav labels e form labels verificati. |

### Accessibilita smart by default

L'accessibilita non deve diventare un carico manuale per l'autore. NODX deve comportarsi come un assistente editoriale: genera cio che puo generare in modo deterministico, segnala solo cio che richiede intenzione umana, e offre scorciatoie semplici per correggere.

**Principio di prodotto:** un documento NODX ben scritto in modo naturale deve produrre output accessibile senza richiedere all'autore di conoscere ARIA, HTML table semantics o WCAG in dettaglio.

| Area | Default automatico | Quando chiedere all'autore |
|---|---|---|
| Headings | Generare outline, ID stabili opzionali, warning sui salti di livello. | Se l'autore forza un salto `h2 -> h4` o duplica ID. |
| TOC/nav | `title` deterministico da ruolo/lingua, `aria-label`, link a heading validi. | Se serve un titolo editoriale diverso o TOC manuale. |
| Tabelle | Prima riga come header con `scope="col"`, caption da `caption` attr o `::caption`. | Se la tabella e senza header, complessa, o richiede scope `row`. |
| Immagini | `decorative="true"` rende `alt=""`; asset mancanti diventano fallback sicuro. | Se immagine informativa manca di `alt`. |
| Figure | Associare `caption` a `figure` nel renderer. | Se manca caption dove il profilo/document type la richiede. |
| Form/field | `label` generato da `name` leggibile se manca. | Se il label generato e ambiguo. |
| Lingua/direzione | `language`, `dir`, e override su span/blocchi. | Se il documento contiene script misti non marcati e il detector segnala ambiguita. |
| Link | Label visibile obbligatoria, URL policy, `title` opzionale. | Se label e generica (`click here`, `link`, `here`). |
| Colori | Temi con contrasto minimo garantito. | Se stile custom viola contrasto minimo. |

**Regole pratiche da implementare:**

1. Renderer HTML deve emettere `scope` sui `<th>` quando l'AST contiene `scope`.
2. Tabelle devono supportare caption ergonomica:
   - `:::table {caption="Control ownership"}`;
   - oppure `::caption` come child.
3. Il validator deve distinguere:
   - errori bloccanti: immagine informativa senza `alt`, URL unsafe, table grid invalida;
   - warning autoriali: heading jump, link label generica, caption mancante;
   - fix automatici: label form da `name`, TOC label default, table header dalla prima riga.
4. Deve esistere un comando/funzione `nodx a11y` o `nodx validate --profile accessibility` che produce un report leggibile e machine-readable.
5. Il playground/editor deve proporre quick fixes: "mark decorative", "add alt", "use first row as header", "add TOC title", "fix heading level".

Questa impostazione mantiene semplice l'authoring: l'autore scrive contenuto naturale, NODX applica semantic defaults, e interviene solo dove manca informazione reale.

### Tabelle, variabili e paginazione come feature 1.0 forti

Dalla verifica del repo emerge che il core attuale e buono, ma per la 1.0 forte servono tre chiusure di prodotto.

**Tabelle**

Le pipe table e le block table sono gia una buona base, ma la 1.0 forte deve definire:

1. caption renderizzata in HTML/PDF/DOCX, da attributo `caption` o child `caption`;
2. `scope="col"`/`scope="row"` preservato nel renderer;
3. griglia coerente obbligatoria con `NODX-E025`;
4. separatore Markdown-compatible `|---|`;
5. policy chiara su escaped pipe (`\|`) dentro celle;
6. decisione esplicita su `colspan`/`rowspan`: supporto limitato o esclusione motivata;
7. import CSV/TSV verso table nodes come tooling, non sintassi core.

La tabella semplice deve restare facilissima:

```
| Metrica | Valore |
|---|---|
| Ricavi | 120K |
```

La tabella professionale deve essere possibile:

```
::table {#controls caption="Control ownership"}
::row
::cell {header="true" scope="col"} Control ::
::cell {header="true" scope="col"} Owner ::
::
::row
::cell Canonical AST hash ::
::cell Platform Engineering ::
::
::
```

### Dataset tabellari, CSV e Parquet/Arrow

**Scope 1.0:** schema tabellare + CSV/TSV import/export.
**Scope post-1.0:** Parquet/Arrow portability — vedi [RFC-0005](docs/RFC-0005-parquet-arrow-portability.md).

NODX puo diventare un ottimo sostituto di CSV quando il problema non e solo "spostare righe e colonne", ma distribuire una tabella con schema, documentazione, provenienza, sicurezza, rendering e metadata. Non deve pero fingere di essere piu efficiente di Parquet per analytics columnar o di CSV per stream grezzi semplicissimi.

**Posizionamento corretto:**

| Formato | Punto forte | Limite | Ruolo NODX |
|---|---|---|---|
| CSV/TSV | Semplice, ubiquo, streamabile. | Niente schema stabile, tipi deboli, encoding/dialect ambigui, niente metadata ricchi. | Sostituto migliore per dataset piccoli/medi documentati e human-readable. |
| Parquet | Columnar, compresso, typed, ottimo analytics. | Poco leggibile, non autoriale, richiede tooling. | Target di export/import per dati tipizzati dentro documenti NODX. |
| Arrow | In-memory/interchange typed. | Non e formato autoriale. | Ponte ideale tra NODX table nodes e data tooling. |
| JSON/NDJSON | Strutturato, web-friendly. | Verboso, schema esterno, meno leggibile per tabelle. | Alternativa machine-facing, non sostituisce table authoring. |

### Table schema profile

Per essere un sostituto serio del CSV, le tabelle NODX devono poter dichiarare schema e tipi in modo leggero:

```
::table {#sales caption="Q1 sales" dataset="sales-q1"}
::columns
::column {name="region" type="string" required="true" label="Region"} ::
::column {name="revenue" type="decimal" unit="EUR" required="true"} ::
::column {name="closed_at" type="date" format="yyyy-mm-dd"} ::
::
| Region | Revenue | Closed at |
|---|---:|---|
| EU | 120000.50 | 2026-03-31 |
| US | 98000.00 | 2026-03-31 |
::
```

Regole:

1. `column.name` e l'identificatore machine-readable stabile.
2. Header/caption sono label umane, non chiavi dati.
3. `type` deve usare un set piccolo e portabile: `string`, `bool`, `int64`, `float64`, `decimal`, `date`, `datetime`, `duration`, `uri`, `json`.
4. `unit`, `format`, `nullable`, `required`, `enum`, `min`, `max` sono metadata opzionali.
5. Il validator deve verificare numero colonne, tipi parseable, required non vuoto, enum valido.
6. Il Canonical AST deve preservare celle come contenuto leggibile, ma l'export data deve produrre valori tipizzati.

### CSV/TSV import/export

La 1.0 forte dovrebbe includere tooling CSV/TSV:

```
nodx import csv sales.csv --schema sales.schema.yaml --caption "Q1 sales" > sales.nodx
nodx export csv sales.nodx --table sales-q1 > sales.csv
nodx export tsv sales.nodx --table sales-q1 > sales.tsv
```

Import CSV deve:
- rilevare dialect in modo conservativo o accettare `--delimiter`, `--quote`, `--header`;
- preservare encoding UTF-8;
- generare table schema se fornito;
- produrre report per righe malformate;
- non eseguire formule spreadsheet;
- neutralizzare CSV injection in export verso spreadsheet (`=`, `+`, `-`, `@`) con policy esplicita.

Export CSV deve:
- usare `column.name` o header label secondo opzione;
- applicare quoting deterministico;
- indicare loss report se celle contengono inline formatting, link, note o contenuto non scalare.

### Parquet/Arrow portability — POST-1.0

**Decisione:** Parquet/Arrow non entrano nella 1.0. Il mapping resta documentato in [RFC-0005](docs/RFC-0005-parquet-arrow-portability.md) e verrà implementato come parte dell'ecosistema dati subito dopo la 1.0.

**Motivazione:** la 1.0 deve mantenere la tesi narrativa "alternativa a Markdown moderna". Aggiungere Parquet/Arrow come release gate amplia lo scope verso il data tooling e ritarda la release di mesi. Schema tabellare + CSV/TSV nella 1.0 sono sufficienti per posizionare NODX come sorgente dataset documentato; Parquet/Arrow arrivano quando ci sono pipeline reali che lo richiedono.

Per riferimento, il mapping previsto (vedi RFC-0005):

Comandi/API previsti:

```
nodx export parquet report.nodx --table sales-q1 -o sales.parquet
nodx export arrow report.nodx --table sales-q1 -o sales.arrow
nodx import parquet sales.parquet --caption "Q1 sales" > sales.nodx
```

Mapping consigliato:

| NODX type | Arrow/Parquet |
|---|---|
| `string` | Utf8 |
| `bool` | Boolean |
| `int64` | Int64 |
| `float64` | Float64 |
| `decimal` | Decimal128 con precision/scale se dichiarate |
| `date` | Date32 |
| `datetime` | Timestamp con timezone esplicita o `timezone="none"` |
| `duration` | Duration |
| `uri` | Utf8 + semantic metadata |
| `json` | Utf8 o extension type documentato |

Metadata NODX da portare in Parquet key-value metadata:
- `nodx.schema`;
- `nodx.table_id`;
- `nodx.caption`;
- `nodx.source_hash`;
- `nodx.column.<name>.label`;
- `nodx.column.<name>.unit`;
- `nodx.column.<name>.semantic_type`;
- `nodx.provenance` se presente.

Questa scelta rende NODX utile in due direzioni:

1. Autori e agenti possono creare dataset leggibili e documentati in NODX.
2. Pipeline dati possono esportare lo stesso contenuto in Parquet/Arrow per analytics senza perdere schema e provenienza.

### Quando NODX sostituisce CSV e quando no

NODX e migliore di CSV per:
- tabelle in documenti tecnici, report, compliance, offerte, audit;
- dataset piccoli/medi che devono essere letti da persone e macchine;
- dati con caption, note, unita, tipi, provenance, sicurezza, package assets;
- output che deve diventare HTML/PDF/DOCX e anche dataset.

CSV resta preferibile per:
- stream enormi append-only;
- pipeline legacy ultra-semplici;
- casi dove non servono schema, metadata, rendering o validazione.

Parquet resta preferibile come formato finale per:
- analytics columnar;
- dataset grandi;
- query engine e lakehouse.

La strategia giusta non e "NODX batte CSV/Parquet sempre", ma: NODX e il formato sorgente/documentale autorevole; CSV/TSV/Parquet/Arrow sono formati di import/export tipizzati e verificabili.

**Variabili**

Le variabili devono supportare due modalita:

| Modalita | Output | Uso |
|---|---|---|
| Semantica | Mantiene `Inline::Var` / placeholder visibile. | NCP, agenti, template non risolti. |
| Risolta | Sostituisce con valore da `vars`/`meta`. | HTML/PDF/DOCX finali. |

Proposta CLI/API:

```
nodx html report.nodx --resolve-vars
nodx html report.nodx --var reviewer="Legal"
nodx validate report.nodx --require-vars
```

Regole:
- `{{name}}` canonicalizza a `{{vars.name}}`;
- `{{meta.title}}` legge metadata;
- variabile mancante e warning nel profilo base, error con `--require-vars` o profilo `template-strict`;
- valori risolti devono essere escapati nel contesto di output;
- niente accesso implicito a env/segreti.

**Paginazione — contratto completo POST-1.0**

**Decisione:** il "Paged Output Contract" completo è rimandato a [RFC-0003](docs/RFC-0003-paged-output-contract.md). La 1.0 supporta:
- `::pagebreak` come manual break (preservato in HTML/PDF/DOCX);
- tema `print` con default sensati (A4, margini, `break-inside: avoid` su tabelle/figure);
- attributi CSS di paginazione nei blocchi `::style` (`@page`, `page-break-*`).

La 1.0 **non** garantisce byte-identical PDF tra motori diversi. Il contratto formale per renderer paged conformi (fixture, livelli, requisiti minimi) arriva in RFC-0003.

La paginazione e concepita su tre livelli, cosi resta semplice ma potente:

| Livello | Meccanismo | Responsabilita |
|---|---|---|
| Manuale | `::pagebreak` | Autore forza una nuova pagina. |
| Dichiarativa | `@page`, `page-break-*`, `orphans`, `widows`, `break-inside` | Tema/stile guida il paged renderer. |
| Automatica | host paged-media contract | Browser/PDF engine calcola pagine in base a misure reali. |

NODX non dovrebbe implementare un motore tipografico completo nella 1.0 core. Deve pero specificare un **Paged Output Contract**:

1. i renderer paged devono rispettare `@page`, `pagebreak`, `break-before/after/inside`, `orphans`, `widows`;
2. il tema `print` deve impostare default sensati per tabelle, figure, note e headings;
3. `pagebreak` deve essere preservato in HTML, PDF bridge e DOCX;
4. il playground/viewer puo simulare pagine, ma la paginazione finale e responsabilita del renderer paged;
5. eventuali differenze di paginazione devono essere fuori dal Canonical AST e non rompere NCP/signature.

Questo e il compromesso giusto: authoring semplice, output professionale, nessuna falsa promessa di byte-identical PDF layout tra motori diversi.

### Editor, WYSIWYG e viewer contract — POST-1.0 (primitive minime in 1.0)

**Decisione:** il contratto editor/WYSIWYG completo è rimandato a [RFC-0004](docs/RFC-0004-editor-contract.md). La 1.0 espone solo **primitive CST minime** (node ranges, stable IDs, local patch, diagnostics mapping) tramite il crate `nodx-cst` e il profilo `editor`.

**Motivazione:** il contratto completo (table editor, package view, accessibility panel, source roundtrip CST) è un piccolo RFC a sé. Imporlo come release gate della 1.0 ritarda la release. Le primitive minime sono sufficienti per costruire una VSCode extension funzionale; il contratto formale matura quando ci sono almeno 2 editor reali che lo implementano.

La struttura NODX e adatta a editor testuali, WYSIWYG e viewer. Le note seguenti restano come guida progettuale per la RFC-0004:

**Punti di forza per editor/viewer:**

1. AST a nodi: naturale per document outline, block editor, inspector laterale.
2. ID stabili: buoni per selezione, commenti, agent mutation e deep links.
3. Attributi strutturati: modificabili da pannelli UI senza parsing testuale fragile.
4. NCP: ottimo per outline, ricerca semantica, agent context e viewer navigation.
5. Fallback children: viewer vecchi possono renderizzare contenuto anche senza conoscere componenti nuovi.
6. ZIP package: viewer puo aprire documento e asset senza rete.

**Contratto minimo per editor WYSIWYG:**

| Funzione editor | Requisito NODX |
|---|---|
| Outline | Derivato da heading + section + TOC/NCP. |
| Inspector blocco | Modifica `id`, classi, attrs, profile-specific attrs. |
| Table editor | Mantiene grid coerente, header/scope, caption. |
| Image editor | Richiede `alt` o `decorative=true`, mostra asset package. |
| Style panel | Scrive token/theme/YAML style, non CSS arbitrario non validato. |
| Source roundtrip | Mantiene semantica e preferibilmente preserva formatting via CST. |
| Validation panel | Mostra diagnostics con quick fixes. |
| Package view | Mostra manifest, assets, digest, warnings sicurezza. |
| Accessibility panel | Mostra solo problemi azionabili, non rumore tecnico. |

**Editing modes consigliati:**

1. `source`: editor testuale con syntax, snippets e diagnostics.
2. `structured`: block editor che manipola AST e rigenera sorgente canonico leggibile.
3. `preview`: rendering sicuro HTML/TUI/PDF bridge.
4. `inspect`: AST/NCP/diagnostics/package/security.
5. `a11y`: checklist accessibile con quick fixes.

**Roundtrip e CST**

Per un WYSIWYG serio serve distinguere due livelli:

- Semantic roundtrip: AST -> source canonico. Sufficiente per generatori e import.
- Lossless/local roundtrip: conserva commenti, spacing, stile di fence, ordine attributi sorgente. Necessario per editor umani.

Quindi il profilo `editor`/CST non e un extra cosmetico: e il modo per rendere NODX modificabile da strumenti visuali senza irritare chi lavora nel sorgente. Per la 1.0 pubblica basta definire il contratto e implementare primitive minime: node ranges, stable IDs, local patch, diagnostics mapping.

### ZIP autoconsistente

Il package ZIP e un differenziatore enorme, ma deve essere una feature d'autore:

```
nodx package create report.nodx --assets assets/ --out report.bundle.nodx
nodx package inspect report.bundle.nodx
nodx package verify report.bundle.nodx
```

La 1.0 dovrebbe includere:
- manifest generato automaticamente;
- digest assets;
- entry document esplicito;
- nessun fetch remoto necessario;
- comando per estrarre report di sicurezza senza estrarre file su disco;
- esempi con immagini, CSS/theme e font locali.

### Release gates aggiornati

Prima di pubblicare la 1.0, il documento deve guidare queste evolutive:

**Nella 1.0 (release gate):**

1. aggiornare grammatica sorgente per NODX-Lite (R2 semplificato, R4, R5 ristretto, R6, R7);
2. implementare vars rapide, attributi light (`#id`) e link attrs in Rust e JS;
3. implementare TOC row override, `toc-hidden` e manual mode;
4. implementare `theme:` con `base`, `web`, `print`;
5. implementare YAML style blocks con paletto pseudo-keys (`print`, `screen`, `dark`, `page`);
6. documentare NODS positive list e design tokens;
7. completare table accessibility: `scope`, caption, escaped pipe policy, fixture;
8. aggiungere risoluzione variabili opzionale e profilo `template-strict`;
9. aggiungere table schema profile con import/export CSV/TSV;
10. supportare pagination minimale (`::pagebreak` + tema `print`);
11. aggiungere comando/report accessibilita con quick fixes;
12. esporre primitive CST minime (node ranges, stable IDs, local patch, diagnostics mapping);
13. aggiungere esempi `layout-fonts`, print, i18n, RTL e package nel quickstart;
14. aggiungere comando `package create`;
15. aggiungere fixture conformance per ogni shorthand;
16. aggiungere fixture negative per ambiguita e input ostile;
17. aggiornare README con "NODX in 5 minuti";
18. rilasciare VSCode starter realmente usabile.

**Post-1.0 (RFC dedicate, non bloccanti):**

- Paged Output Contract completo → [RFC-0003](docs/RFC-0003-paged-output-contract.md);
- Editor/WYSIWYG contract completo → [RFC-0004](docs/RFC-0004-editor-contract.md);
- Parquet/Arrow portability → [RFC-0005](docs/RFC-0005-parquet-arrow-portability.md).

### Roadmap ricalibrata

**Fase A — Format hardening prima della 1.0**

1. NODX-Lite: short fences, attrs light, vars rapide, table separator Markdown-compatible.
2. Navigation: TOC automatico/manuale, row override, hidden entries, accessible labels.
3. Links: link title shorthand e post-link attrs.
4. Themes: `theme:` e temi `none/base/web/print`.
5. Styles: CSS/NODS positivo + YAML style authoring.
6. i18n/RTL/accessibility con fixture e test.
7. Tables: caption, scope, escaped pipe policy, grid validation.
8. Data tables: schema, typed cells, CSV/TSV import/export, Parquet/Arrow portability.
9. Variables: semantic mode, resolved mode, strict template validation.
10. Pagination: manual breaks + declarative CSS + host automatic contract.

**Fase B — Tooling minimo prima della 1.0**

1. CLI aggiornata: validate, html, ncp, package create/inspect/verify, watch.
2. Rust e JS in conformance sul nuovo formato.
3. VSCode starter con syntax, snippets, diagnostics e preview.
4. Playground con esempi progressivi.
5. Accessibility panel/report con quick fixes.
6. Editor/WYSIWYG contract: AST inspector, table editor, package view, CST mapping.
7. README, quickstart e examples index.

**Fase C — Ecosistema subito dopo 1.0**

1. Python package.
2. Markdown import wizard.
3. Theme registry / `.nodt`.
4. GitHub Action.
5. Signature profile portabile.
6. Presentation profile completo.

---

## 12. Governance della 1.0 — scope, breaking, metriche, costi

Questa sezione chiude il piano evolutivo definendo cosa **non** entra nella 1.0, come gestire le breaking change rispetto alla bozza corrente, come misurare il successo della release e una prima stima di costo per fase. Senza questi paletti, il piano resta una lista di desideri.

### 12.1 Non-goals 1.0 (riassunto operativo)

Le seguenti aree **non** sono release gate per la 1.0. Restano valide come direzione futura, con spec dedicate dove serve.

| Area | Stato 1.0 | Rimandato a |
|---|---|---|
| Profili shorthand (`profiles: core+rich`) | Non incluso | Eventuale 1.x se richiesto |
| Paged Output Contract completo | Solo `::pagebreak` + tema `print` | [RFC-0003](docs/RFC-0003-paged-output-contract.md) |
| Editor/WYSIWYG/CST contract completo | Solo primitive CST minime in `nodx-cst` | [RFC-0004](docs/RFC-0004-editor-contract.md) |
| Parquet/Arrow portability | Schema tabellare + CSV/TSV sì; Parquet/Arrow no | [RFC-0005](docs/RFC-0005-parquet-arrow-portability.md) |
| Firma digitale portabile | `reserved` nel manifest, profilo `signature` non normativo | 1.x |
| Presentation profile completo (slide/deck) | Tema `presentation` base sì; profilo slide completo no | RFC-0006 futura |
| Registry online temi | Solo `.nodt` locale | Post-1.0 |
| Layout grid avanzato | Blocchi base `columns/row/card/callout` sì; grid 2D avanzato no | Post-1.0 |
| Plugin Neovim/Emacs/JetBrains | Solo VSCode in 1.0 | Post-1.0 |
| GitHub Action ufficiale | Post-1.0 | Post-1.0 |

**Regola operativa:** se durante l'implementazione una feature 1.0 si dimostra più grande del previsto, prima di estendere il calendario va valutato lo spostamento in non-goals.

### 12.2 Breaking changes rispetto alla bozza corrente

La bozza attuale ha già esempi `.nodx` nel repo (vedi `examples/`). La 1.0 introduce cambi sintattici che richiedono una strategia di migrazione esplicita.

**Breaking change principali della 1.0:**

| # | Cambiamento | Impatto | Mitigazione |
|---|---|---|---|
| B1 | Forma corta `::nome` introdotta accanto a `:::nome` | Nessun break (additivo) | `:::` resta valido |
| B2 | Forma light `# Titolo #intro` accanto a `{#intro}` | Nessun break (additivo) | `{#id}` resta canonica |
| B3 | Front matter implicito quando `schema`/`type`/`profiles` mancano | Nessun break (additivo) | Documenti con front matter completo restano validi |
| B4 | Vars `{{name}}` come alias di `{{vars.name}}` | Nessun break (additivo) | `{{vars.name}}` resta canonica |
| B5 | Separatore tabella `\|---\|` accettato | Nessun break (additivo) | `\| - \|` resta valido |
| B6 | Link `[label](url){attrs}` | Nessun break (additivo) | `[label](url)` resta valida |
| B7 | YAML style blocks con pseudo-keys ristrette | Nuovo, non rompe CSS esistente | CSS raw nei blocchi `::style` resta valido |
| B8 | `theme:` nel front matter | Nuovo opzionale | Default `theme: none` se assente |
| B9 | TOC con `toc=`, `toc-hidden=`, `mode="manual"` | Nuovo opzionale | TOC esistente continua a funzionare |
| B10 | Table schema profile (colonne tipizzate) | Nuovo opzionale | Tabelle Markdown-style senza schema restano valide |

**Strategia di migrazione:**

1. **Tutte le aggiunte 1.0 sono additive**: i documenti scritti con la bozza corrente continuano a parsare. Questa è la regola d'oro.
2. **Comando `nodx migrate`**: tooling che riscrive un `.nodx` esistente nella forma più idiomatica 1.0 (front matter ridotto, `::nome`, `{#id}` → `#id` dove applicabile). Opt-in, non destructive.
3. **Schema version dichiarata**: `schema: nodx/1.0` resta valida. Documenti senza `schema:` assumono `nodx/1.0` (era già la direzione del front matter implicito).
4. **Esempi del repo**: dopo il freeze della 1.0, gli esempi in `examples/` vengono riscritti nella forma idiomatica 1.0 come reference autoritative.
5. **Pre-1.0 → 1.0 freeze**: tra l'ultima bozza pre-1.0 e la 1.0 finale, garantire una finestra di 4 settimane di sola correzione bug, senza nuove feature sintattiche.

### 12.3 Metriche di successo per la 1.0

Una 1.0 "forte" si misura su risultati osservabili, non su sensazioni. Le seguenti metriche definiscono il successo della release a 6 e 12 mesi dal lancio.

**Metriche di adozione (target 6 mesi post-1.0):**

| Metrica | Target | Misurazione |
|---|---|---|
| Parser conformi indipendenti | ≥ 2 (Rust + JS) passano 100% conformance suite | CI repo `conformance/` |
| VSCode extension installs | ≥ 1.000 | Marketplace stats |
| Stelle GitHub repo | ≥ 500 | GitHub |
| Documenti `.nodx` su GitHub (search) | ≥ 100 file in repo terzi | GitHub code search |
| Esempi reali end-to-end nel repo | ≥ 8 (minimal, web, print, package, i18n, RTL, table-data, agent-workflow) | `examples/` |

**Metriche di qualità (target alla 1.0):**

| Metrica | Target | Misurazione |
|---|---|---|
| Conformance fixture passate | 100% su Rust e JS | CI |
| Fixture negative/hostile coperte | ≥ 50 casi (XSS, YAML hostile, ZIP slip, URL unsafe) | `crates/*/tests/` |
| Coverage codice core | ≥ 85% su `nodx-core`, `nodx-validate`, `nodx-render-html` | tarpaulin/coverage tools |
| Latency parse documento medio (~10KB) | < 5ms su Rust release | benchmark suite |
| A11y report su esempi standard | 0 errori bloccanti | `nodx a11y` |

**Metriche narrative (target alla 1.0):**

1. Un utente che conosce Markdown deve essere produttivo in NODX in **< 30 minuti** dalla lettura del Quickstart.
2. Un agente AI deve generare un `.nodx` valido conforme al profilo `core+rich` partendo da un prompt naturale in **un singolo round-trip** (verificabile su test set documentato).
3. Conversione Markdown→NODX su 50 documenti reali (presi da repo open source) deve produrre output valido senza intervento manuale nel **≥ 90%** dei casi.

Senza queste metriche, "1.0 forte" resta soggettivo. Con esse, è verificabile.

### 12.4 Stima costi per fase

Le tre fasi della roadmap (A/B/C) hanno costi molto diversi. Mescolarle nella stessa pianificazione genera slittamenti. La stima sotto è in **persona-settimana** (PS) di lavoro focalizzato, assumendo un team piccolo (2-3 dev full-time + 1 doc/designer part-time).

**Fase A — Format hardening prima della 1.0**

| Voce | Stima PS | Note |
|---|---|---|
| NODX-Lite (R2 semplificato, R4, R6, R7) | 4 | Parser Rust + JS, fixture |
| Attributi light `#id` (R5 ristretto) | 1 | Solo `#id` finale |
| TOC controllabile (`toc=`, manual, hidden) | 3 | Include accessibility labels |
| Themes `theme:` + base/web/print | 4 | CSS + design tokens |
| YAML style blocks (con paletto pseudo-keys) | 3 | Parser duale CSS/YAML |
| i18n/RTL fixture e test | 3 | CJK + RTL + mixed |
| Tables: caption, scope, escaped pipe, grid validation | 2 | |
| Table schema profile + CSV/TSV import/export | 4 | Schema profile, tooling CLI |
| Variabili: semantic/resolved, `--require-vars` | 2 | |
| Pagination 1.0 minimale (`::pagebreak` + tema `print`) | 2 | Contratto completo in RFC-0003 |
| Accessibility smart defaults + `nodx a11y` | 3 | Quick fixes |
| **Totale Fase A** | **~31 PS** | ≈ 8 settimane con 4 persone |

**Fase B — Tooling minimo prima della 1.0**

| Voce | Stima PS | Note |
|---|---|---|
| CLI aggiornata (validate, html, ncp, package, watch) | 3 | |
| Conformance Rust e JS sul nuovo formato | 3 | |
| VSCode extension (syntax, snippets, diagnostics, preview) | 5 | MVP usabile |
| Playground con esempi progressivi | 2 | |
| Primitive CST minime (`nodx-cst`) | 4 | Solo basi per editor (no contratto completo, vedi RFC-0004) |
| `nodx package create/inspect/verify` | 3 | |
| README + Quickstart + cheat sheet | 3 | Documentazione narrativa |
| **Totale Fase B** | **~23 PS** | ≈ 6 settimane con 4 persone |

**Totale 1.0 (Fase A + Fase B):** ~54 PS ≈ **14 settimane di calendario** con team di 4 persone, ovvero **~3,5 mesi di sviluppo focalizzato** + 4 settimane di stabilizzazione = **~4,5 mesi totali**.

**Fase C — Ecosistema post-1.0 (riferimento)**

| Voce | Stima PS |
|---|---|
| Python package `nodx-py` | 6 |
| Markdown→NODX wizard | 4 |
| Theme registry + `.nodt` esterno | 3 |
| RFC-0003 implementazione (Paged Output Contract) | 6 |
| RFC-0004 implementazione (Editor Contract completo) | 8 |
| RFC-0005 implementazione (Parquet/Arrow) | 5 |
| GitHub Action ufficiale | 2 |
| Signature profile portabile | 5 |
| Plugin Neovim/Emacs | 6 |
| **Totale Fase C** | **~45 PS** | ≈ 11 settimane, da distribuire su 6-9 mesi |

**Avvertenze sulla stima:**

- Le PS escludono review, design, riunioni, imprevisti. Moltiplicatore realistico: ×1,4.
- La Fase A ha rischio più alto: cambi sintattici hanno cascading impact su parser, validator, renderer, fixture, esempi, doc.
- La Fase B è più prevedibile (tooling sopra spec stabile).
- La Fase C può essere parallelizzata; non richiede team unico.

### Certificazione finale aggiornata

Rivedo il verdetto precedente: le scorciatoie autoriali non vanno considerate accessorie. Per una 1.0 pubblica davvero forte, CSS/YAML, vars rapide, attributi light (solo `#id`), link attrs, TOC controllabile, temi, layout/font/colori, i18n/RTL, sicurezza, table schema e package ZIP autoconsistente devono essere parte del piano principale.

La condizione e non introdurli come convenienze vaghe: ogni feature deve avere grammatica stretta, AST canonico, conformance test, negative/security test e documentazione con esempi. Le aree più ambiziose (Paged Output Contract, Editor Contract completo, Parquet/Arrow) restano spec separate (RFC-0003/0004/0005) e arrivano post-1.0 senza bloccare la release.

Cosi NODX puo essere facile come Markdown nel caso comune e molto piu potente nei casi professionali, **e la 1.0 può essere effettivamente spedita in ~4,5 mesi** invece di restare un piano di 12+ mesi.

---

*Documento di analisi — 11 maggio 2026 (revisione: piano 1.0 calibrato)*
