import { describe, expect, it } from "vitest";
import { localeFromLanguage, optionLabels, parseLocale } from "./i18n";

describe("i18n", () => {
  it("保存値として有効なlocaleだけを受け付ける", () => {
    expect(parseLocale("ja")).toBe("ja");
    expect(parseLocale("en")).toBe("en");
    expect(parseLocale("fr")).toBeNull();
    expect(parseLocale(null)).toBeNull();
  });

  it("browser languageから日本語または英語を選ぶ", () => {
    expect(localeFromLanguage("ja-JP")).toBe("ja");
    expect(localeFromLanguage("en-US")).toBe("en");
    expect(localeFromLanguage("fr-FR")).toBe("en");
    expect(localeFromLanguage(undefined)).toBe("en");
  });

  it("レンダリングバックエンドの表示名を日英で持つ", () => {
    expect(optionLabels.ja.renderBackend.gpu).toBe("GPU");
    expect(optionLabels.en.renderBackend.skia).toBe("Skia");
  });
});
