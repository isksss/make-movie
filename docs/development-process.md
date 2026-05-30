# 開発プロセス

## 基本方針

Issue Driven Development を採用します。

```text
Issue
↓
feature branch
↓
実装
↓
PR
↓
review
↓
develop
↓
main
```

## Agent 分離

- 実装 Agent と Review Agent は分ける。
- Review Agent と Merge Agent は分ける。
- 可能な限り、実装、テスト、レビュー、マージを分離する。

## Issue テンプレート

```text
Title

Goal

Requirements

Acceptance Criteria

Verification Steps

References
```
