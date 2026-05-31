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

## Issue 再確認

外部からIssueが追加される運用に対応するため、PRを3本作成するごとにopen Issueを再確認します。

```bash
gh issue list --state open --limit 100 --json number,title,url,createdAt,updatedAt
```

新しいIssueが追加されていた場合は、現在の作業キューへ反映し、優先順位を見直します。
