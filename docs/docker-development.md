# Docker 開発・テスト環境

CI とローカル検証の差分を小さくするため、Ubuntu ベースの Docker Compose 環境を用意しています。

## 前提

- Docker
- Docker Compose

コンテナは `ubuntu:24.04` を固定し、GitHub Actions の `ubuntu-latest` が更新されてもローカル検証が急に変わらないようにします。
Node.js は現在の CI に合わせて 22 系、pnpm は `10.24.0` を既定にしています。

## 初回ビルド

```bash
docker compose build dev
```

Node.js の major version を変える場合は build arg を指定します。

```bash
NODE_MAJOR=26 docker compose build dev
```

## シェル

```bash
docker compose run --rm dev bash
```

初回起動時は entrypoint が GUI と TypeScript SDK の依存関係を `pnpm install --frozen-lockfile` で準備し、Playwright Chromium もインストールします。
依存準備を省略したい場合は次のように実行します。

```bash
MM_DOCKER_BOOTSTRAP=0 docker compose run --rm dev bash
```

## 一括検証

```bash
docker compose run --rm dev bash scripts/verify-all.sh
```

このコマンドは Rust workspace、GUI、Tauri backend、Plugin SDK、Playwright E2E をまとめて確認します。
`dotnet` は dev image に含めていないため、C# SDK build は `scripts/verify-all.sh` の既存挙動どおり skip されます。

## 個別検証

```bash
docker compose run --rm dev cargo test --workspace
docker compose run --rm dev corepack pnpm --dir apps/mm-gui lint
docker compose run --rm dev corepack pnpm --dir apps/mm-gui test
docker compose run --rm dev corepack pnpm --dir apps/mm-gui e2e
docker compose run --rm dev cargo test --manifest-path apps/mm-gui/src-tauri/Cargo.toml
```

## キャッシュ

Compose は repository を `/workspace` に bind mount します。
`target`、`node_modules`、`.pnpm-store`、Playwright の実行結果は既存の `.gitignore` により Git 管理外です。
依存関係を完全に作り直す場合は次を実行します。

```bash
docker compose down
docker compose build --no-cache dev
```

## CI との差分

- CI は GitHub Actions の `ubuntu-latest`、Docker は再現性のため `ubuntu:24.04` 固定です。
- CI は workflow 内で `pnpm install` と `playwright install --with-deps chromium` を実行します。
- Docker は OS 依存を image build 時に入れ、プロジェクト依存と Playwright browser を entrypoint で準備します。
- Docker 内の Tauri E2E は Xvfb 上で実行し、WebKitGTK sandbox と DMA-BUF renderer はテスト専用に無効化します。
- Docker の Tauri E2E は WebKitGTK の sandbox と namespace 制約を避けるため、dev service に `SYS_ADMIN` capability と `seccomp=unconfined` を付与します。この compose service は開発・テスト専用です。
- C# SDK build は `dotnet` がある環境のみ実行します。

CI の Node.js や Ubuntu version を変更する場合は、`.github/workflows/ci.yml`、`.mise.toml`、`compose.yaml` の `NODE_MAJOR` 既定値を同じ Issue で更新してください。
