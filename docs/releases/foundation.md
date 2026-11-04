Project foundation for the future SIP desktop application.

- Rust workspace, Tauri 2 desktop shell and Svelte/TypeScript frontend.
- Generated IPC contract and explicit native engine availability.
- macOS and Windows build preparation.

This development scaffold cannot make or receive calls. baresip, account storage,
contacts, call history and calendar integration are not implemented. macOS uses
ad-hoc signing; Windows installers are unsigned. Audio/SIP interoperability and
production distribution have not been qualified.
