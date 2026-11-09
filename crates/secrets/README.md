# secrets

Cross-platform facade over OS credential stores. Initial targets: Windows Credential Manager and Linux Secret Service. No silent plaintext fallback. APIs should use opaque handles and avoid returning secret values to UI code.
