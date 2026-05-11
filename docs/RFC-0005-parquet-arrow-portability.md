# NODX-RFC-0005 — Parquet/Arrow Portability (Draft)

> Stato: Draft, post-1.0
> Dipende da: NODX-RFC-0001 (table nodes 1.0)
> Scopo: definire export/import deterministico tra table nodes NODX tipizzati e formati columnar (Parquet, Arrow).

---

## 1. Motivazione

La 1.0 di NODX include table nodes semantici, header/scope e validazione della griglia. Schema tabellare tipizzato, import/export CSV/TSV e portabilità Parquet/Arrow restano tooling/draft post-1.0: questa RFC definisce il mapping proposto affinché NODX possa diventare sorgente documentale di dataset tipizzati senza perdere schema, semantica e provenienza nella conversione.

## 2. Non-goals

- Sostituire Parquet come formato analytics.
- Implementare query engine columnar in NODX.
- Supportare tutte le estensioni Arrow.

## 3. Posizionamento

NODX è il **sorgente documentale autorevole**. Parquet/Arrow sono **formati di export tipizzati** per analytics. Il flusso canonico è:

```
NODX (sorgente)  →  Parquet/Arrow (analytics)
   ↑                        |
   └─ reimport via Arrow ───┘ (opzionale, lossy su contenuto inline)
```

## 4. Mapping tipi NODX → Arrow/Parquet

| NODX type | Arrow/Parquet |
|---|---|
| `string` | Utf8 |
| `bool` | Boolean |
| `int64` | Int64 |
| `float64` | Float64 |
| `decimal` | Decimal128 con precision/scale dichiarate |
| `date` | Date32 |
| `datetime` | Timestamp con timezone esplicita o `timezone="none"` |
| `duration` | Duration |
| `uri` | Utf8 + semantic metadata `nodx.semantic_type=uri` |
| `json` | Utf8 o Arrow extension type documentato |

## 5. Metadata preservati

NODX deve scrivere in Parquet key-value metadata:

- `nodx.schema` — versione schema NODX
- `nodx.table_id` — ID stabile della tabella
- `nodx.caption` — caption originale
- `nodx.source_hash` — digest del sorgente NODX
- `nodx.column.<name>.label` — label umana
- `nodx.column.<name>.unit` — unità di misura
- `nodx.column.<name>.semantic_type` — semantic type esteso
- `nodx.provenance` — provenance se dichiarata nel documento

## 6. CLI proposta

```
nodx export parquet report.nodx --table sales-q1 -o sales.parquet
nodx export arrow report.nodx --table sales-q1 -o sales.arrow
nodx import parquet sales.parquet --caption "Q1 sales" > sales.nodx
```

## 7. Regole di import

L'import Parquet → NODX produce una tabella con schema dichiarato. Limitazioni:

- inline formatting (link, mark, code) **non** è ricostruibile da Parquet;
- caption viene presa da `nodx.caption` se presente, altrimenti dal parametro `--caption`;
- celle vengono renderizzate come testo formatted secondo `format`/`unit` della colonna.

L'import è quindi **lossy** rispetto a contenuto autoriale, ma **lossless** rispetto a dati tipizzati.

## 8. Conformance fixture richieste

- `parquet/roundtrip-basic.parquet` — tipi base.
- `parquet/roundtrip-decimal.parquet` — decimal con precision/scale.
- `parquet/roundtrip-datetime-tz.parquet` — datetime con timezone.
- `parquet/metadata-preserved.parquet` — verifica key-value metadata.

## 9. Roadmap

Promozione a stabile dopo:
- implementazione in `nodx-export` e `nodx-import`;
- almeno 2 pipeline reali (un export verso lakehouse, un import da dataset esistente);
- fixture conformance complete;
- documentazione "Come usare NODX come sorgente dataset".
