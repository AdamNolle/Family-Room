# Implementation approach

The accepted direction is native clients over one Rust catalog, encryption, permission, provider, and sync implementation. The Apple UI prototype has been replaced at the app entry point by the real library. Existing HTML/JSX explorations remain design references.

See [architecture](docs/ARCHITECTURE.md) for the implemented boundaries and protocol, [UI design](docs/UI_DESIGN.md) for Cozy Glass, and [feature register](docs/FEATURE_REGISTER.md) for the complete scope. UI screens must show actual catalog and transfer state, never simulated members, provider connections, quota, recognition results, or successful exports.

The production release remains gated on privacy boundaries, provider authorization, background reliability, platform interoperability, and creative rendering. Local test success is not evidence that those external integrations are complete.
