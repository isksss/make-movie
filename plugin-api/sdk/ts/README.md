# mm-sdk-ts

TypeScript SDK for make-movie plugins.

## Install

```bash
npm install mm-sdk-ts
```

pnpm:

```bash
pnpm add mm-sdk-ts
```

## Example

```ts
import { definePlugin, metadataToJson } from "mm-sdk-ts";

export default definePlugin({
  metadata: () =>
    metadataToJson({
      name: "my-plugin",
      version: "0.1.0",
      category: "utility",
    }),
  initialize: () => undefined,
  shutdown: () => undefined,
});
```

## Publish

```bash
pnpm build
npm publish --dry-run
npm publish --access public
```
