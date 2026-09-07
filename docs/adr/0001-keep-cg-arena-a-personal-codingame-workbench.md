# Keep CG Arena a personal CodinGame workbench

CG Arena optimizes for one bot author evaluating improvements locally for one CodinGame challenge. CodinGame integration is the primary experience, while custom-command referees remain an advanced compatibility path; this focus avoids turning the project into a generic tournament service.

## Consequences

- An Arena is controlled by one author in a trusted local or LAN environment.
- Multi-user accounts, authentication, untrusted-code sandboxing, public tournament hosting, and CodinGame account/submission integration are out of scope.
- Execution currently uses one embedded worker on the local machine.
- Distributed workers are not current product scope, but the worker boundary remains explicit so trusted remote workers can be considered long-term.
