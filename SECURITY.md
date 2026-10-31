# Security policy

This Rust branch supports the current 2.x portable application on Windows 10/11 x64. The removed .NET implementation is retained in Git history and is not maintained in this branch.

Report problems in [this repository's issues](https://github.com/Jiang-bc/pinwin/issues). For sensitive details, use GitHub's private vulnerability reporting if enabled; avoid posting private window titles or personal configuration in public reports.

PinWin normally runs with the caller's permissions. Windows can deny access to elevated or protected applications. Explicit CLI handle/title actions change the specified external windows directly; managed GUI windows are tracked separately and their original state is restored when unpinned or on orderly exit. Abrupt termination cannot guarantee restoration.

Custom icons and cursors are local files loaded through Windows APIs. Keep the portable directory writable only by trusted users. Runtime dependencies are recorded in Cargo.lock; image conversion dependencies are isolated in the development-only artwork tool.
