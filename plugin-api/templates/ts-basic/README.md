# TypeScript Plugin Template

TypeScript で make-movie Plugin を作る最小テンプレートです。

## Install

外部 repository で使う場合は SDK を npm から追加します。

```bash
npm install mm-sdk-ts
```

pnpm:

```bash
pnpm add mm-sdk-ts
```

この repository 内の検証では `file:../../sdk/ts` を使います。

## Build / Test

```bash
corepack pnpm install
corepack pnpm test
corepack pnpm build
```

TypeScript Plugin を Runtime で読み込む場合は、最終成果物を `plugin-api/plugin.wit` と互換の WASM Component に変換します。変換 toolchain はPluginの種類に応じて選択します。
