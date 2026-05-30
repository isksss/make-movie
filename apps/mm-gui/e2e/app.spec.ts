import { expect, test } from "@playwright/test";

test("主要ペインとプレビュー描画を確認できる", async ({ page }) => {
  await page.goto("/");

  await expect(page.getByText("make-movie")).toBeVisible();
  await expect(page.getByRole("region", { name: "Assets" })).toBeVisible();
  await expect(page.getByRole("region", { name: "Preview" })).toBeVisible();
  await expect(page.getByRole("region", { name: "Property" })).toBeVisible();
  await expect(page.getByRole("region", { name: "Timeline" })).toBeVisible();

  const canvas = page.getByLabel("Rendered preview");
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

test("プレビュー操作とタイムライン選択がUIに反映される", async ({ page }) => {
  await page.goto("/");

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
  await height.fill("240");
  await page.getByLabel("Rotation").fill("15");
  await page.getByLabel("Opacity").fill("0.5");
  await expect(height).toHaveValue("240");
  await expect(page.getByLabel("Rotation")).toHaveValue("15");
  await expect(page.getByLabel("Opacity")).toHaveValue("0.5");
});

test("PropertyからCrop/Mask/Fitを編集しUndo/Redoできる", async ({ page }) => {
  await page.goto("/");

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
