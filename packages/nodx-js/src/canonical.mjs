export function canonicalJson(doc) {
  const { body, meta, schema } = doc;
  return canonicalStringify({ body, meta, schema });
}

export function canonicalStringify(value) {
  return JSON.stringify(sortValue(value));
}

export function sortValue(value) {
  if (Array.isArray(value)) return value.map(sortValue);
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.keys(value).sort().map((key) => [key, sortValue(value[key])]));
  }
  return value;
}
