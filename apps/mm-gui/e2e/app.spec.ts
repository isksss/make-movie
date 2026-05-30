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

  await useEnglish(page);

  await expect(page.getByText("Assets")).toBeVisible();
  await expect(page.getByText("Property", { exact: true })).toBeVisible();

  await page.getByRole("combobox", { name: "Language" }).selectOption("ja");
  await expect(page.getByText("アセット")).toBeVisible();
  await expect(page.getByText("プロパティ")).toBeVisible();
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
  await expect(page.getByLabel("Fit")).toHaveValue("contain");

  await page.getByLabel("Crop X").fill("10");
  await page.getByLabel("Crop Y").fill("20");
  await page.getByLabel("Crop Width").fill("320");
  await page.getByLabel("Crop Height").fill("180");
  await page.getByLabel("Mask").selectOption("rounded_rect");
  await page.getByLabel("Fit").selectOption("blur_background");

  await expect(page.getByLabel("Crop Width")).toHaveValue("320");
  await expect(page.getByLabel("Mask")).toHaveValue("rounded_rect");
  await expect(page.getByLabel("Fit")).toHaveValue("blur_background");

  await page.getByRole("button", { name: "Undo" }).click();
  await expect(page.getByLabel("Fit")).toHaveValue("contain");

  await page.getByRole("button", { name: "Redo" }).click();
  await expect(page.getByLabel("Fit")).toHaveValue("blur_background");
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
  await expect(transition).toHaveValue("none");

  await transition.selectOption("wipe");
  await transitionDuration.fill("1.2");
  await expect(transition).toHaveValue("wipe");
  await expect(transitionDuration).toHaveValue("1.2");

  await page.getByRole("button", { name: "Undo" }).click();
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

  await property.selectOption("opacity");
  await easing.selectOption("ease_in_out");
  await keyframe2Time.fill("1.5");
  await keyframe2Value.fill("0.25");
  await expect(property).toHaveValue("opacity");
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
  await expect(property).toHaveValue("opacity");
});
