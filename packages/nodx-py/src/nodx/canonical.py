import json


def canonical_json(doc):
    return canonical_stringify(
        {"body": doc["body"], "meta": doc["meta"], "schema": doc["schema"]}
    )


def canonical_stringify(value):
    return json.dumps(sort_value(value), ensure_ascii=False, separators=(",", ":"))


def sort_value(value):
    if isinstance(value, list):
        return [sort_value(item) for item in value]
    if isinstance(value, dict):
        return {key: sort_value(value[key]) for key in sorted(value)}
    return value
