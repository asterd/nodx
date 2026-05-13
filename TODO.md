**NODX** (Node-Oriented Document eXchange) è un formato di documento testuale-first, orientato a nodi semantici, progettato per essere leggibile da umani, tool e agenti AI. Mantiene la semplicità di Markdown ma aggiunge garanzie forti su struttura, sicurezza, portabilità e determinismo.

### Panoramica profonda del repo (stato attuale, ~maggio 2026)

- **Obiettivo principale**: Rimpiazzare Markdown in contesti dove serve **determinismo** (Canonical AST JSON stabile), **sicurezza** (fail-closed per input non fidati), **packaging** di asset locali, **navigazione semantica** stabile (ID espliciti, TOC), e una proiezione compatta per agenti (NCP — NODX Compact Projection).
- **Specifica normativa**: `NODX-RFC-0001.md` (versione 1.0 finalizzata di recente). Definisce sintassi, front-matter safe YAML subset, profili di conformance (`core`, `rich`, `style`, `package`, `agent-read`, ecc.), limiti di risorse, policy URL/sicurezza, package manifest, ecc.
- **Implementazione di riferimento**: Rust (`crates/`), `#![forbid(unsafe_code)]`. Include parser, validator, renderer HTML/TUI, package handler, NCP projector, CLI (`nodx` binary), supporto signing (sperimentale), ecc.
- **Portabilità**: Parser indipendenti in JS (`packages/nodx-js`) e Python. Suite di conformance molto robusta con fixture per AST, NCP, diagnostics, rendering, security, package.
- **Packaging**: Supporto nativo per `.nodx` come ZIP (`application/nodx+zip`). Rileva dal magic bytes (PK\x03\x04). Include manifest, digest per integrità, safe path policy, limiti su size/compressione/nested ZIP. Esempi in `examples/extended-showcase-bundled.nodx`.
- **Sicurezza**: Fail-closed forte — no script execution, no network di default, escaping contestuale, resource limits stretti (es. 64 MiB source, nesting depth 32, ecc.). Threat model e policy in `SECURITY.md` e docs.
- **Altre feature**:
  - Lite syntax (simile a Markdown) + full node syntax (`::component {attrs} ... ::`).
  - Variabili, attributi, custom components con fallback.
  - Safe style blocks (NODS?).
  - Export preview (PDF, DOCX, PPTX).
  - Editor support starters.
- **Maturità**: 1.0 stabile su parsing, AST, package reader, NCP. Alcune parti (signing completo, agent-mutate, editor CST lossless) sono reserved/future profiles.

Il progetto è ben strutturato, con enfasi su conformance, testabilità e interop. Sembra pensato per ecosistemi document-oriented moderni (docs, report, knowledge base, AI agents).

### 1. Gestione del formato "pacchetto" come ZIP senza compressione

**È sensato?** Sì, molto sensato per il tuo caso d'uso attuale, e allinea bene con NODX.

**Pro**:
- **Semplicità e velocità**: ZIP senza compressione (store method) è essenzialmente un container concatenato di file con header. Lettura/scrittura ultra-veloce, nessun overhead di deflate.
- **Debuggabilità**: Puoi aprire il `.nodx` con qualsiasi unzipper o tool ZIP e vedere tutto chiaramente (manifest + main document + assets).
- **Determinismo**: Più facile garantire che lo stesso contenuto produca esattamente lo stesso byte stream (importante per hash, signing, conformance).
- **NODX lo supporta nativamente**: Il package reader gestisce già ZIP, con manifest `nodx-package/1.0`, digest checks, safe extraction policy. Usare store mode non rompe nulla (la spec permette compression ratio limits proprio per prevenire zip bomb, ma store evita il problema).
- **Uso come "pacchetto multi-file"**: Perfetto per unire più file in un singolo artefatto portabile senza perdere la natura testuale del documento principale.

**Contro / Quando aggiungere compressione**:
- **Dimensione**: Se hai asset grandi (immagini, font, media) o documenti molto voluminosi, la dimensione esplode. Compress (deflate) riduce banda/storage.
- **Trasferimento**: Su rete o download, senza compressione paghi di più.
- **Performance in alcuni contesti**: Su storage lento o bandwidth limitata, la compressione aiuta.

**Raccomandazione**:
- **Mantieni store (no compressione) come default** per sviluppo, editing, CI, versioning, e casi dove velocità/debug contano di più (es. documenti interni, knowledge base, tool chain).
- **Aggiungi opzione per compressione** (deflate level 6-9) per distribuzione finale/export ("build" o "publish" command). NODX package già gestisce compression ratio checks.
- Puoi rilevare/negoziare: se il pacchetto supera X MiB uncompressed → offri versione compressed.
- Per integrità + anti-tampering, combina con digest/manifest (già in NODX) o signing (vedi Q2).

In sintesi: **ottima scelta attuale**, non rompere ciò che funziona. Aggiungi compressione come feature opzionale.

### 2. "Marchiare" il file in modo sicuro (immutabile, legato all'autore, sola lettura "umana/certificata")

Vuoi:
- File **immutabile** (non modificabile senza rompere la certificazione).
- Legato indissolubilmente a un utente/autore.
- Rimanga leggibile come testo (o quasi).
- Senza sistemi troppo complicati.
- Resistente a modifiche (tranne fotocopie + OCR, che è il limite fisico inevitabile).

**Approccio raccomandato che si integra bene con NODX (senza rompere l'esistente)**:

NODX ha già supporto per **signing** nella reference impl (menzionato in README e RFC come reserved profile `signature`, ma con infrastruttura presente). Usa quello come base.

#### Soluzione ibrida semplice e robusta:

1. **Struttura del pacchetto** (ZIP):
   - `main.nodx` → il documento testuale (o entry point).
   - `manifest.json` (o embedded in front-matter) con metadata autore (user ID, pubkey, timestamp, ecc.).
   - `signature.jws` (o detached signature) che firma un **hash canonico** dell'intero contenuto rilevante (es. Canonical AST + assets ordinati + manifest).

2. **Cosa firmare** (per immutabilità forte):
   - Non firmare i byte raw grezzi del ZIP (troppo fragile a ri-packaging).
   - Firma il **Canonical AST JSON** (deterministico per design in NODX) + hash degli asset (file paths + content digest, già supportati).
   - O un "package digest tree" (Merkle-like) per permettere verifica parziale.
   - Includi nel signed payload: autore (did, username, pubkey fingerprint), timestamp, profile `signature`, claims ("this document is read-only / authored by X").

3. **Implementazione minima**:
   - Usa **JWS** (JSON Web Signature) con ECDSA (P-256 o Ed25519) — standard, librerie everywhere (Rust: `jsonwebtoken` o `ecdsa` crate; già menzionato in NODX).
   - Aggiungi comandi CLI: `nodx package sign ... --key private.pem` e `nodx package verify`.
   - Nel package: metti la firma in un file dedicato (`_nodx/signature.jws`).
   - Per "sola lettura umana": il documento testuale rimane leggibile. La firma protegge semanticamente (verifica che AST non sia cambiato).
   - Per legare all'utente: usa Web of Trust leggero, DID, o semplicemente chiave pubblica associata a identità (es. GitHub, email + cert, o chiave in un registry semplice).

4. **Verifica**:
   - Processor verifica firma → se OK, mostra badge "Certified by @user — immutable".
   - Se modificato (anche ri-zippato senza compressione), firma fallisce.
   - Rimane resistente a OCR/foto perché la firma è sul contenuto semantico strutturato, non sui pixel.

**Vantaggi**:
- Non rompe compatibilità: documenti senza firma continuano a funzionare (profile `signature` optional).
- Integra con package esistente (manifest + digests).
- Sicuro e standard (no crypto inventata).
- "Indissolubile" a livello crittografico (firma privata dell'autore).
- Leggibile: il testo principale non è criptato.

**Alternative / Estensioni**:
- **Timestamping authority** (es. OpenTimestamps) per prova di esistenza temporale.
- **Blockchain anchor** (hash sul chain) per immutabilità pubblica (più complicato, ma forte).
- Per "sola lettura" UI: renderer che, se firma OK, disabilita edit o mostra warning.
- Se vuoi evitare ZIP esterno: embed firma in un blocco speciale alla fine del documento testuale (es. `::signature { ... }` con JWS base64), ma package è più pulito per asset.

**Potenziali gotcha**:
- Gestisci ri-packaging deterministico (ordine file nel ZIP, compression=store).
- Revoca chiavi (problema classico — usa short-lived cert o CRL semplice).
- UX: rendi signing one-command e verifica automatica nel viewer.

Questa approccio è **sicuro, non troppo complicato**, e si sposa perfettamente con la filosofia NODX (determinismo + package + profili). Se vuoi, posso aiutarti a sketchare codice Rust/CLI o schema del manifest firmato. 

Fammi sapere dettagli sul tuo flusso (es. chi è l'"autore" — utente finale, server, ecc.) per raffinare!



**No, non è una scelta troppo stringente per NODX 1.0.** È una delle scelte più coerenti e difendibili del formato.

### Perché è sensata (e forte)

- **Sicurezza fail-closed** è un pilastro del progetto (vedi SECURITY.md e sezione 19 della RFC). Consentire fetch remoti di default aprirebbe a:
  - Tracking / fingerprinting dell’utente che apre il documento.
  - Dipendenza da disponibilità esterna (link rotto = documento degradato).
  - Attacchi indiretti (malicious image che sfrutta decoder del browser, o future 0-day).
  - Violazione del principio “il documento è completamente portabile e deterministico”.
- Il modello **package-first** (ZIP con assets locali + manifest + digest) è uno dei grandi punti di forza rispetto a Markdown. Permette integrità verificabile, offline-first, signing futuro, ecc.
- Per i **link testuali** (`[testo](https://...)`) invece gli http/https sono già consentiti (solo per navigazione, non per asset).

**In sintesi**: per un formato che vuole essere “Markdown ma con garanzie forti”, il divieto su remote media è corretto per la baseline 1.0.

### Quando ha senso rilassarlo

Sì, è utile dare la possibilità **in modo controllato**, soprattutto per casi reali:
- Documenti live / dashboard
- Report aziendali con immagini da CDN interno
- Knowledge base condivise
- Preview veloci durante authoring

### Come implementerei l’estensione (senza rompere nulla)

#### 1. Nuovo profilo opzionale
```yaml
profiles:
  optional:
    - remote-assets   # o remote-media
```

- Se il profilo non è dichiarato e richiesto → comportamento attuale (rifiuto).
- Se dichiarato in `optional` → processori che lo supportano possono abilitarlo.

#### 2. Policy granulare nella sezione URL (estensione di Sezione 19)

Aggiungi nella tabella Reference-Kind Policy:

| Reference kind | Allowed (con remote-assets) |
|----------------|-----------------------------|
| Asset / Image / Media / Embed | safe package-relative + http/https (con policy) + data: (già permesso) |

**Regole di sicurezza per remote assets** (da applicare obbligatoriamente):

- Solo `https` (http rifiutato).
- **Allow-list** di host/domains (configurazione del processor/host, es. `trusted-cdns = ["*.mycompany.com", "cdn.example.org"]`). Default vuoto (quindi niente).
- Opzione “any” solo in contesti esplicitamente trusted (es. viewer desktop dell’azienda).
- **Content-Security-Policy** forte generata nell’HTML renderer.
- **Timeout + size limit** sul fetch (es. 10s, 5-10 MiB per risorsa).
- **Cache aggressiva** con ETag/Last-Modified e fallback offline al package (se presente).
- **Digest opzionale** nell’attributo: `src="https://..." sha256="..."` → verifica integrità dopo download.
- Diagnostic dedicato (es. `NODX-E028 Remote asset fetched` o warning se non in allow-list).

#### 3. Sintassi (retrocompatibile)

```nodx
::image {src="https://cdn.example.com/photo.jpg" alt="..." cache="true" timeout="8s"}

::media {src="https://..." sha256="base64url-digest" ...}
```

O un attributo generico:
`remote="allowed"` / `remote-policy="strict"`

#### 4. Comportamento del processor

- **CLI baseline** (`nodx html ...`): rifiuta di default, a meno di flag `--allow-remote-assets --trusted-hosts "*.internal"`.
- **Viewer/Editor integrati**: opzione nelle impostazioni dell’app.
- **Package mode**: può includere una sezione `remote-assets` nel manifest con allow-list consigliata dall’autore.
- **NCP / AST**: mantieni l’URL originale nell’attributo `src`, aggiungi campo `resolved` o `fetched` solo nel rendering.

#### 5. Altre best practice

- Durante il packaging (`nodx package ...`) → opzione per **embed** automatico di remote assets referenziati (scarica e include nel ZIP).
- Per firma (future profile signature): il signed payload può includere gli hash degli assets remoti approvati.
- Warning visibile nel rendering: “This document loads remote images from trusted sources”.

### Raccomandazione finale

- **Mantieni il default strict** (è un vantaggio competitivo).
- Aggiungi il profilo `remote-assets` (o `network`) come **optional** nella 1.1 o come estensione post-1.0.
- Implementalo con allow-list + digest opzionali + cache → resta molto più sicuro di Markdown/HTML normale.

Vuoi che ti sketchi esattamente come modificherei la sezione 19 della RFC o il codice Rust del URL validator? O preferisci prima vedere pro/contro di alternative (es. solo CDN allow-list globale vs per-documento)?