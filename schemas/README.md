# Burr JSON Schemas

These JSON Schema Draft 2020-12 files describe Burr's public JSON contracts:

- `burr.design-data.v1.schema.json`
- `burr.rulepack.v1.schema.json`
- `burr.receipt.v2.schema.json`

The Rust CLI remains the authoritative evaluator. The schemas are published
with Burr's source distributions so editors, generators, and CI integrations
can validate documents before invoking `burr check`.

Schema validity does not predict a passing receipt. Cross-item and filesystem
invariants such as unique rule, part, and feature ids, ordered range bounds,
file freshness, and rulepack compatibility remain CLI checks.

Design-data feature objects intentionally allow additional fields because
custom rulepacks may consume domain-specific metadata. Rulepack syntax is
closed: unknown top-level, rule, or selector fields are rejected so a typo
cannot silently weaken a check.

Intent strings remain extensible in the schema. At runtime, only Burr's
documented non-mechanical intents are coverage-exempt; unknown intent values
remain coverage-required until an evaluated rule checks the feature.
