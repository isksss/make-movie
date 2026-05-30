# AGENTS.md

## 目的

このリポジトリで作業するエージェント向けの必須ルールを定義します。

## 必須ルール

- 最終回答、Issue、PR、コメント、ドキュメントは日本語で書く。
- `mm.toml` を唯一の Source of Truth として扱う。
- GUI 専用プロジェクト形式は作らない。
- Core 機能は Plugin で置き換えない。
- Plugin は追加機能専用とする。
- 変更は Issue 単位で行う。
- ユーザー指示なしに secret、deploy、package publish、force push を行わない。
- `PLAN.local.md` と `codingplan.local.md` はローカル下書きとして扱い、通常は変更しない。

## 詳細

- アーキテクチャ: `docs/architecture.md`
- 開発プロセス: `docs/development-process.md`
- Git 運用: `docs/git-workflow.md`
- Plugin: `docs/plugin-system.md`
- Project 構成: `docs/project-structure.md`
- MVP: `docs/mvp.md`
- 検証: `docs/verification.md`
