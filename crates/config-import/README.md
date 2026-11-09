# config-import

Parsers convert untrusted external representations into validated domain profiles. Import is a pure parsing/validation operation: it must not start a connection or change OS state.

Planned modules: share links, QR payloads, Xray JSON, WireGuard, subscriptions, and NPV formats. Each parser needs valid, invalid, oversized, and version-mismatch fixtures.
