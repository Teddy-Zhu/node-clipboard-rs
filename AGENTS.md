# Repository Guidelines

## Project Structure & Module Organization
This repository is a Node.js + Rust N-API addon (`@teddyzhu/clipboard`).
- Rust core: `src/lib.rs`, `Cargo.toml`, `build.rs`
- JS/TS entry and types: `index.js`, `index.d.ts`
- Automated tests: `__test__/index.spec.ts` (AVA)
- Manual/integration scripts: `tests/*.js` (clipboard listener/watch scenarios)
- CI and release workflow: `.github/workflows/CI.yml`

Keep logic changes in Rust first, then expose APIs via `index.js` and `index.d.ts` together.

## Build, Test, and Development Commands
- `yarn build`: release build via NAPI (`napi build --platform --release`)
- `yarn build:debug`: debug build for local iteration
- `yarn test`: run AVA test suite
- `yarn lint`: run `oxlint` on JS/TS
- `yarn format`: format JS/TS, Rust, and TOML (`prettier`, `cargo fmt`, `taplo`)
- `cargo fmt -- --check`: Rust format check (same as CI lint stage)

Example local loop: `yarn lint && yarn test && yarn build:debug`.

## Coding Style & Naming Conventions
- Follow `.editorconfig`: spaces, 2-space indentation, LF, UTF-8.
- JS/TS style is enforced by Prettier: no semicolons, single quotes, trailing commas, max width 120.
- Rust style is enforced by `rustfmt` (`tab_spaces = 2`).
- Use clear API naming consistent with existing exports (`getClipboardText`, `setClipboardText`, `ClipboardManager`).
- Keep file names lowercase with underscores only when existing patterns already use them (e.g., `test_watch.js`).

## Testing Guidelines
- Primary framework: AVA with TypeScript support through `@oxc-node/core/register`.
- Add tests in `__test__/index.spec.ts` for new public API behavior.
- Name tests by behavior, e.g. `test('ClipboardManager - 文本操作', ...)`.
- `tests/*.js` are useful manual checks for platform/display-specific behavior; do not rely on them alone for regressions.

## Commit & Pull Request Guidelines
- Commit messages in history are short and imperative (`fix test`, `support workflow_dispatch`).
- Release commits are version-only (`v0.0.6`), which CI uses to trigger publish.
- PR checklist:
1. Explain what changed and why.
2. Note affected platforms (Linux/macOS/Windows).
3. Include verification summary (`yarn lint`, `yarn test`, optional build target).
4. Link the related issue when applicable.

Avoid bundling unrelated refactors with release/version bumps.
