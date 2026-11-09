# SSH engine

First target: OpenSSH client with local dynamic forwarding (SOCKS5). Use discrete process arguments, avoid shell invocation, support password/key auth without exposing secrets in process listings, and verify listener/process readiness before reporting Connected.
