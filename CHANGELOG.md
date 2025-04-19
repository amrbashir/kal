# Changelog

## [Unreleased]

## Added

- New icon
- Report panicking errors in a dialog box instead of silentely closing the app.
- Paths in configuration file can now use `~` and `$HOME` and will be resolved to the user home directory.

## Fixed

- Fix jumping text when a result item's primary text is empty
- Fix explorer navigation expanding when opening directories

## Changed

- Use `%LOCALAPPDATA%` for installation as `%APPDATA%` is used for runtime data and cache
- Shortcut will be placed inside `Start Menu/Programs` directly instead of ``Start Menu/Programs/kal`
- _`[Calculator]`_ Set default score to 200 for its result so it shows up higher

## Fixed

- _`[Calculator]`_ Show error only if being queried directly

## [0.2.0] - 2025-1-31

### Added

- Check WebView2 availability on startup and propmt for installation.

### Fixed

- _`[Everything]`_ Fix terminal window popup when querying

### Changed

- _`[Calculator]`_ Use `sci-calc` crate to fix issues with older implementation and add new features like `sqrt`, `tan`, `cos`...etc

## [0.1.0] - 2025-1-29

- Inital Release
