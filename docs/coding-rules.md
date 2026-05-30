# コーディングルール

## 共通

- `mm.toml` を唯一の Source of Truth として扱う。
- GUI 専用 project format は作らない。
- Core 機能を Plugin で置き換えない。
- Issue の要求範囲に関係しないリファクタは行わない。
- エラーは `anyhow::Context` などで原因と対象パスが分かる形にする。

## Rust

- `cargo fmt` の結果を正とする。
- `cargo clippy --workspace --all-targets -- -D warnings` を通す。
- Core の型は `serde` で TOML 往復できる形を保つ。
- ファイルパスは `Path` / `PathBuf` を使い、文字列連結で組み立てない。
- FFmpeg、TTS、Plugin など外部プロセスや外部 I/O はテストで差し替えられる境界を置く。

## TypeScript / React

- `pnpm --dir apps/mm-gui lint` と `pnpm --dir apps/mm-gui test` を通す。
- GUI は表示と操作のみを担当し、Project の永続化形式は `mm.toml` に集約する。
- 状態管理は既存の Zustand store を優先して使う。
- UI 文言は日本語・英語切替に追従するよう `src/ui/i18n.ts` に集約する。
- E2E で参照する操作名はアクセシブルな `title` / `aria-label` / label と一致させる。

## Plugin

- `plugin-api/plugin.wit` を唯一の ABI 契約とする。
- Plugin は追加機能専用とし、Core の必須編集機能を置き換えない。
- Plugin manager は registry を前提にせず、GitHub / GitLab / URL / Local の解決を扱う。

## ドキュメント

- Issue、PR、コメント、ドキュメントは日本語で書く。
- 新しい開発規約を追加した場合は、必要に応じて `AGENTS.md` から導線を張る。
