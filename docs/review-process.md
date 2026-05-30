# レビュープロセス

## 基本方針

レビューはバグ、回帰、仕様未達、検証不足を優先して確認します。

## Agent 分離

- 実装 Agent と Review Agent は分ける。
- Review Agent と Merge Agent は分ける。
- 可能な限り、実装、テスト、レビュー、マージを別 Agent が担当する。

## レビュー観点

- Issue の Goal / Requirements / Acceptance Criteria を満たしているか。
- `mm.toml` が Source of Truth として維持されているか。
- GUI が Core / Tauri command へ責務を委譲できているか。
- Plugin が Core 機能を置き換えていないか。
- CLI / GUI の日本語・英語対応を壊していないか。
- 既存テストが通り、変更範囲に応じたテストが追加されているか。

## PR 確認

PR 本文は日本語で、以下を含めます。

- 概要
- 関連 Issue
- 変更内容
- 動作確認
- 既知の問題
- レビューポイント

マージ前に `docs/verification.md` の該当コマンドを通します。
