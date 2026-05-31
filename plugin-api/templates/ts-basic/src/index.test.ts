import { type MmPlugin } from "mm-sdk-ts";
import plugin from "./index.js";

const typedPlugin: MmPlugin = plugin;
const metadata: string = typedPlugin.metadata();

if (!metadata.includes('"name":"ts-basic"')) {
  throw new Error("metadata name is missing");
}
