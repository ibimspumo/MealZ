# Codex App Server protocol snapshot

This directory is generated from `codex-cli 0.144.1` using:

```sh
codex app-server generate-json-schema --out docs/codex-protocol/schema --experimental
```

MealZ uses these schemas as a compatibility snapshot for its JSONL stdio client. They document the exact local contract; the Rust implementation remains intentionally small and parses only the methods and notifications used by the product.
