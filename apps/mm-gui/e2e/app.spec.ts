import { expect, test } from "@playwright/test";
import type { Page } from "@playwright/test";

async function useEnglish(page: Page) {
  await page.getByRole("combobox", { name: /^(言語|Language)$/ }).selectOption("en");
}

test("主要ペインとプレビュー描画を確認できる", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByText("make-movie")).toBeVisible();
  await expect(page.getByRole("region", { name: "アセット" })).toBeVisible();
  await expect(page.getByRole("region", { name: "プレビュー" })).toBeVisible();
  await expect(page.getByRole("region", { name: "プロパティ" })).toBeVisible();
  await expect(page.getByRole("region", { name: "タイムライン" })).toBeVisible();

  const canvas = page.getByLabel("レンダリングプレビュー");
  await expect(canvas).toBeVisible();
  await expect
    .poll(async () =>
      canvas.evaluate((element) => {
        const target = element as HTMLCanvasElement;
        const context = target.getContext("2d");
        if (!context || target.width === 0 || target.height === 0) {
          return false;
        }
        const data = context.getImageData(0, 0, target.width, target.height).data;
        for (let index = 0; index < data.length; index += 4) {
          if (data[index] !== 16 || data[index + 1] !== 20 || data[index + 2] !== 23) {
            return true;
          }
        }
        return false;
      }),
    )
    .toBe(true);
});

test("Languageで日本語と英語を切り替えられる", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByText("アセット")).toBeVisible();
  await expect(page.getByText("プロパティ")).toBeVisible();
  await expect(page.getByLabel("マスク").locator("option[value='rounded_rect']")).toHaveText(
    "角丸四角",
  );
  await expect(page.getByLabel("フィット").locator("option[value='blur_background']")).toHaveText(
    "ぼかし背景",
  );
  await expect(page.getByLabel("プロバイダー").locator("option[value='aivis_speech']")).toHaveText(
    "AivisSpeech",
  );

  await useEnglish(page);

  await expect(page.getByText("Assets")).toBeVisible();
  await expect(page.getByText("Property", { exact: true })).toBeVisible();
  await expect(
    page
      .getByRole("combobox", { name: "Mask", exact: true })
      .locator("option[value='rounded_rect']"),
  ).toHaveText("Rounded rectangle");
  await expect(
    page
      .getByRole("combobox", { name: "Fit", exact: true })
      .locator("option[value='blur_background']"),
  ).toHaveText("Blur background");
  await expect(
    page
      .getByRole("combobox", { name: "Provider", exact: true })
      .locator("option[value='coeiro_ink']"),
  ).toHaveText("CoeiroInk");

  await page.getByRole("combobox", { name: "Language" }).selectOption("ja");
  await expect(page.getByText("アセット")).toBeVisible();
  await expect(page.getByText("プロパティ")).toBeVisible();
  await expect(page.getByLabel("マスク").locator("option[value='rounded_rect']")).toHaveText(
    "角丸四角",
  );
});

test("プレビュー操作とタイムライン選択がUIに反映される", async ({ page }) => {
  await page.goto("/");
  await useEnglish(page);

  await page.getByRole("button", { name: "Next frame" }).click();
  await expect(page.getByText("0.03s")).toBeVisible();

  await page
    .getByRole("region", { name: "Timeline" })
    .getByRole("button", { name: "Intro Image" })
    .click();
  await expect(page.getByLabel("Layer")).toHaveValue("Intro Image");

  await page.getByLabel("Start").fill("1.2");
  await expect(page.getByText("Intro Image")).toBeVisible();

  await page.getByLabel("Playback rate").selectOption("2");
  await expect(page.getByLabel("Playback rate")).toHaveValue("2");

  const loop = page.getByRole("button", { name: "Loop" });
  await expect(loop).toHaveAttribute("aria-pressed", "false");
  await loop.click();
  await expect(loop).toHaveAttribute("aria-pressed", "true");
});

test("プロパティ編集をUndo/Redoできる", async ({ page }) => {
  await page.goto("/");
  await useEnglish(page);

  await page
    .getByRole("region", { name: "Timeline" })
    .getByRole("button", { name: "Intro Image" })
    .click();
  await page.getByLabel("Start").fill("1.2");
  await expect(page.getByLabel("Start")).toHaveValue("1.2");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(page.getByLabel("Start")).toHaveValue("0.5");

  await page.getByRole("button", { name: "Redo" }).click();
  await expect(page.getByLabel("Start")).toHaveValue("1.2");
});

test("PropertyからGroupを編集しUndo/Redoできる", async ({ page }) => {
  await page.goto("/");
  await useEnglish(page);

  await page
    .getByRole("region", { name: "Timeline" })
    .getByRole("button", { name: "Subtitle" })
    .click();
  const group = page.getByRole("combobox", { name: "Group" });
  await expect(group).toHaveValue("");

  await group.selectOption("opening");
  await expect(group).toHaveValue("opening");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(group).toHaveValue("");

  await page.getByRole("button", { name: "Redo" }).click();
  await expect(group).toHaveValue("opening");
});

test("Textレイヤーのスタイルを編集しUndo/Redoできる", async ({ page }) => {
  await page.goto("/");
  await useEnglish(page);

  const property = page.getByRole("region", { name: "Property" }).locator(".property-grid");
  await property.locator("textarea").first().fill("Updated title");
  await property.getByLabel("Font Size").fill("72");
  await property.getByLabel("Text Color").fill("#ffcc00");
  await property.getByLabel("Text Gradient").check();
  await property.getByLabel("Start Color").fill("#ff0000");
  await property.getByLabel("End Color").fill("#0000ff");
  await property.getByLabel("Direction").selectOption("horizontal");
  await property.getByLabel("Text Align").selectOption("left");
  await property.getByLabel("Stroke Width").fill("3");
  await property.getByLabel("Shadow X").fill("4");
  await property.getByLabel("Shadow Blur").fill("6");

  await expect(property.getByLabel("Layer")).toHaveValue("Updated title");
  await expect(property.getByLabel("Font Size")).toHaveValue("72");
  await expect(property.getByLabel("Direction")).toHaveValue("horizontal");
  await expect(property.getByLabel("Text Align")).toHaveValue("left");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(property.getByLabel("Text Gradient")).toBeChecked();
  await expect(property.getByLabel("Shadow Blur")).toHaveValue("0");

  await page.getByRole("button", { name: "Redo" }).click();
  await expect(property.getByLabel("Text Gradient")).toBeChecked();
  await expect(property.getByLabel("Shadow Blur")).toHaveValue("6");
});

test("選択中クリップを現在時刻でCutできる", async ({ page }) => {
  await page.goto("/");
  await useEnglish(page);

  await page
    .getByRole("region", { name: "Timeline" })
    .getByRole("button", { name: "Intro Image" })
    .click();
  await page.getByLabel("Seek").fill("2.5");
  await page.getByRole("button", { name: "Cut" }).click();

  await expect(
    page.getByRole("region", { name: "Timeline" }).getByRole("button", {
      name: "Intro Image (2)",
    }),
  ).toBeVisible();
  await expect(page.getByLabel("Start")).toHaveValue("2.5");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(
    page.getByRole("region", { name: "Timeline" }).getByRole("button", {
      name: "Intro Image (2)",
    }),
  ).toHaveCount(0);

  await page.getByRole("button", { name: "Redo" }).click();
  await expect(
    page.getByRole("region", { name: "Timeline" }).getByRole("button", {
      name: "Intro Image (2)",
    }),
  ).toBeVisible();
});

test("選択中クリップをDuplicate/Deleteできる", async ({ page }) => {
  await page.goto("/");
  await useEnglish(page);

  const timeline = page.getByRole("region", { name: "Timeline" });
  await timeline.getByRole("button", { name: "Intro Image" }).click();
  await page.getByRole("button", { name: "Duplicate" }).click();

  await expect(timeline.getByRole("button", { name: "Intro Image Copy" })).toBeVisible();
  await expect(page.getByLabel("Layer")).toHaveValue("Intro Image Copy");
  await expect(page.getByLabel("Start")).toHaveValue("7.5");

  await page.getByRole("button", { name: "Delete" }).click();
  await expect(timeline.getByRole("button", { name: "Intro Image Copy" })).toHaveCount(0);

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(timeline.getByRole("button", { name: "Intro Image Copy" })).toBeVisible();

  await page.getByRole("button", { name: "Redo" }).click();
  await expect(timeline.getByRole("button", { name: "Intro Image Copy" })).toHaveCount(0);
});

test("PropertyからTransformを編集しUndo/Redoできる", async ({ page }) => {
  await page.goto("/");
  await useEnglish(page);

  await page
    .getByRole("region", { name: "Timeline" })
    .getByRole("button", { name: "Intro Image" })
    .click();
  const width = page.getByRole("spinbutton", { name: "Width", exact: true });
  await expect(width).toHaveValue("840");
  await width.fill("420");
  await expect(width).toHaveValue("420");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(width).toHaveValue("840");

  await page.getByRole("button", { name: "Redo" }).click();
  await expect(width).toHaveValue("420");

  const height = page.getByRole("spinbutton", { name: "Height", exact: true });
  const rotation = page.getByRole("spinbutton", { name: "Rotation", exact: true });
  const opacity = page.getByRole("spinbutton", { name: "Opacity", exact: true });
  await height.fill("240");
  await rotation.fill("15");
  await opacity.fill("0.5");
  await expect(height).toHaveValue("240");
  await expect(rotation).toHaveValue("15");
  await expect(opacity).toHaveValue("0.5");
});

test("PropertyからCrop/Mask/Fitを編集しUndo/Redoできる", async ({ page }) => {
  await page.goto("/");
  await useEnglish(page);

  await page
    .getByRole("region", { name: "Timeline" })
    .getByRole("button", { name: "Intro Image" })
    .click();
  await expect(page.getByRole("combobox", { name: "Fit", exact: true })).toHaveValue("contain");

  await page.getByRole("spinbutton", { name: "Crop X", exact: true }).fill("10");
  await page.getByRole("spinbutton", { name: "Crop Y", exact: true }).fill("20");
  await page.getByRole("spinbutton", { name: "Crop Width", exact: true }).fill("320");
  await page.getByRole("spinbutton", { name: "Crop Height", exact: true }).fill("180");
  await page.getByRole("combobox", { name: "Mask", exact: true }).selectOption("rounded_rect");
  await page.getByRole("spinbutton", { name: "Mask Radius", exact: true }).fill("24");
  await page.getByRole("combobox", { name: "Fit", exact: true }).selectOption("blur_background");

  await expect(page.getByRole("spinbutton", { name: "Crop Width", exact: true })).toHaveValue(
    "320",
  );
  await expect(page.getByRole("combobox", { name: "Mask", exact: true })).toHaveValue(
    "rounded_rect",
  );
  await expect(page.getByRole("spinbutton", { name: "Mask Radius", exact: true })).toHaveValue(
    "24",
  );
  await expect(page.getByRole("combobox", { name: "Fit", exact: true })).toHaveValue(
    "blur_background",
  );

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(page.getByRole("combobox", { name: "Fit", exact: true })).toHaveValue("contain");

  await page.getByRole("button", { name: "Redo" }).click();
  await expect(page.getByRole("combobox", { name: "Fit", exact: true })).toHaveValue(
    "blur_background",
  );

  await page.getByRole("combobox", { name: "Mask", exact: true }).selectOption("svg");
  await page.getByLabel("Mask Path").fill("media/mask/window.svg");
  await expect(page.getByRole("combobox", { name: "Mask", exact: true })).toHaveValue("svg");
  await expect(page.getByLabel("Mask Path")).toHaveValue("media/mask/window.svg");
});

test("PropertyからTrimを編集しUndo/Redoできる", async ({ page }) => {
  await page.goto("/");
  await useEnglish(page);

  await page
    .getByRole("region", { name: "Timeline" })
    .getByRole("button", { name: "Voice Audio" })
    .click();

  const trimStart = page.getByRole("spinbutton", { name: "Trim Start", exact: true });
  const trimEnd = page.getByRole("spinbutton", { name: "Trim End", exact: true });
  await trimStart.fill("0.25");
  await trimEnd.fill("1.5");
  await expect(trimStart).toHaveValue("0.25");
  await expect(trimEnd).toHaveValue("1.5");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(trimEnd).toHaveValue("0");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(trimStart).toHaveValue("0");

  await page.getByRole("button", { name: "Redo" }).click();
  await expect(trimStart).toHaveValue("0.25");
});

test("PropertyからTransitionを編集しUndo/Redoできる", async ({ page }) => {
  await page.goto("/");
  await useEnglish(page);

  await page
    .getByRole("region", { name: "Timeline" })
    .getByRole("button", { name: "Intro Image" })
    .click();
  const transition = page.getByRole("combobox", { name: "Transition", exact: true });
  const transitionDuration = page.getByRole("spinbutton", {
    name: "Transition Duration",
    exact: true,
  });
  const wipeShape = page.getByRole("combobox", { name: "Wipe Shape", exact: true });
  const wipeRadius = page.getByRole("spinbutton", { name: "Wipe Radius", exact: true });
  const wipeBorderWidth = page.getByRole("spinbutton", {
    name: "Wipe Border Width",
    exact: true,
  });
  const wipeShadowX = page.getByRole("spinbutton", { name: "Wipe Shadow X", exact: true });
  const wipeShadowY = page.getByRole("spinbutton", { name: "Wipe Shadow Y", exact: true });
  const wipeShadowBlur = page.getByRole("spinbutton", {
    name: "Wipe Shadow Blur",
    exact: true,
  });
  await expect(transition).toHaveValue("none");

  await transition.selectOption("wipe");
  await transitionDuration.fill("1.2");
  await wipeShape.selectOption("rounded_rect");
  await wipeRadius.fill("28");
  await wipeBorderWidth.fill("3");
  await wipeShadowX.fill("4");
  await wipeShadowY.fill("5");
  await wipeShadowBlur.fill("6");
  await expect(transition).toHaveValue("wipe");
  await expect(transitionDuration).toHaveValue("1.2");
  await expect(wipeShape).toHaveValue("rounded_rect");
  await expect(wipeRadius).toHaveValue("28");
  await expect(wipeBorderWidth).toHaveValue("3");
  await expect(wipeShadowX).toHaveValue("4");
  await expect(wipeShadowY).toHaveValue("5");
  await expect(wipeShadowBlur).toHaveValue("6");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(wipeShadowBlur).toHaveValue("0");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(wipeShadowY).toHaveValue("0");

  await page.getByRole("button", { name: "Redo" }).click();
  await expect(wipeShadowY).toHaveValue("5");

  await page.getByRole("button", { name: "Redo" }).click();
  await expect(wipeShadowBlur).toHaveValue("6");

  for (let index = 0; index < 7; index += 1) {
    await page.getByRole("button", { name: "Undo" }).click();
  }
  await expect(transitionDuration).toHaveValue("0.5");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(transition).toHaveValue("none");

  await page.getByRole("button", { name: "Redo" }).click();
  await expect(transition).toHaveValue("wipe");
});

test("PropertyからEffectを編集しUndo/Redoできる", async ({ page }) => {
  await page.goto("/");
  await useEnglish(page);

  await page
    .getByRole("region", { name: "Timeline" })
    .getByRole("button", { name: "Intro Image" })
    .click();
  const effect = page.getByRole("combobox", { name: "Effect", exact: true });
  const effectAmount = page.getByRole("spinbutton", { name: "Effect Amount", exact: true });
  const effectDuration = page.getByRole("spinbutton", {
    name: "Effect Duration",
    exact: true,
  });
  await expect(effect).toHaveValue("none");

  await effect.selectOption("blur");
  await effectAmount.fill("2.5");
  await effectDuration.fill("1.2");
  await expect(effect).toHaveValue("blur");
  await expect(effectAmount).toHaveValue("2.5");
  await expect(effectDuration).toHaveValue("1.2");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(effectDuration).toHaveValue("0.5");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(effectAmount).toHaveValue("1");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(effect).toHaveValue("none");

  await page.getByRole("button", { name: "Redo" }).click();
  await expect(effect).toHaveValue("blur");
});

test("PropertyからKeyframeを編集しUndo/Redoできる", async ({ page }) => {
  await page.goto("/");
  await useEnglish(page);

  await page
    .getByRole("region", { name: "Timeline" })
    .getByRole("button", { name: "Intro Image" })
    .click();
  const property = page.getByRole("combobox", { name: "Keyframe Property", exact: true });
  const easing = page.getByRole("combobox", { name: "Easing", exact: true });
  const keyframe2Time = page.getByRole("spinbutton", {
    name: "Keyframe 2 Time",
    exact: true,
  });
  const keyframe2Value = page.getByRole("spinbutton", {
    name: "Keyframe 2 Value",
    exact: true,
  });
  await expect(property).toHaveValue("none");

  await property.selectOption("crop_width");
  await easing.selectOption("ease_in_out");
  await keyframe2Time.fill("1.5");
  await keyframe2Value.fill("0.25");
  await expect(property).toHaveValue("crop_width");
  await expect(easing).toHaveValue("ease_in_out");
  await expect(keyframe2Time).toHaveValue("1.5");
  await expect(keyframe2Value).toHaveValue("0.25");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(keyframe2Value).toHaveValue("1");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(keyframe2Time).toHaveValue("1");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(easing).toHaveValue("linear");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(property).toHaveValue("none");

  await page.getByRole("button", { name: "Redo" }).click();
  await expect(property).toHaveValue("crop_width");
});

test("TTS編集でProvider/Speaker/Text/Speed/Pitch/Emotionを編集しUndo/Redoできる", async ({
  page,
}) => {
  await page.goto("/");
  await useEnglish(page);

  const ttsEditor = page.locator(".tts-editor");
  const provider = ttsEditor.getByRole("combobox", { name: "Provider", exact: true });
  const speaker = ttsEditor.getByLabel("Speaker");
  const text = ttsEditor.locator("textarea");
  const speed = ttsEditor.getByRole("spinbutton", { name: "Speed", exact: true });
  const pitch = ttsEditor.getByRole("spinbutton", { name: "Pitch", exact: true });
  const emotion = ttsEditor.getByRole("textbox", { name: "Emotion", exact: true });

  await provider.selectOption("coeiro_ink");
  await expect(provider).toHaveValue("coeiro_ink");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(provider).toHaveValue("voicevox");

  await page.getByRole("button", { name: "Redo" }).click();
  await expect(provider).toHaveValue("coeiro_ink");

  await speaker.fill("四国めたん");
  await text.fill("更新後の文章");
  await speed.fill("1.4");
  await pitch.fill("0.2");
  await emotion.fill("happy");

  await expect(speaker).toHaveValue("四国めたん");
  await expect(text).toHaveValue("更新後の文章");
  await expect(speed).toHaveValue("1.4");
  await expect(pitch).toHaveValue("0.2");
  await expect(emotion).toHaveValue("happy");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(emotion).toHaveValue("neutral");

  await page.getByRole("button", { name: "Redo" }).click();
  await expect(emotion).toHaveValue("happy");
  await expect(provider).toHaveValue("coeiro_ink");
});
