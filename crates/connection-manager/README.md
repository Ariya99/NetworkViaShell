# README

Owns connection orchestration and lifecycle state. Coordinates preparation, engine startup, readiness checks, reconnect policy, stop, and compensating cleanup. Stop must be idempotent and events/errors sanitized.
