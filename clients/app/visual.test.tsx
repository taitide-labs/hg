import { expect, open, test } from "nsh/test";

const SCREENSHOT_DIR = "../../target/visual-check";

test("captures light and dark discover and dual-mode player screenshots", async () => {
  const light = await open(".", {
    width: 1240,
    height: 800,
    colorScheme: "light",
  });
  await light.waitFor("云端之上：千金归来");
  await light.screenshot(`${SCREENSHOT_DIR}/01-discover-light.png`);

  await light.press("云端之上：千金归来");
  await light.waitFor("本地离线 · HEVC");
  await light.screenshot(`${SCREENSHOT_DIR}/02-player-cinema-light.png`);

  await light.press("切至短剧模式");
  await light.waitFor("切至电影模式");
  await light.screenshot(`${SCREENSHOT_DIR}/03-player-shortdrama-light.png`);

  const dark = await open(".", {
    width: 1240,
    height: 800,
    colorScheme: "dark",
  });
  await dark.waitFor("云端之上：千金归来");
  await dark.screenshot(`${SCREENSHOT_DIR}/04-discover-dark.png`);

  await dark.press("云端之上：千金归来");
  await dark.waitFor("本地离线 · HEVC");
  await dark.screenshot(`${SCREENSHOT_DIR}/05-player-cinema-dark.png`);

  expect(dark.texts()).toContain("本地离线 · HEVC");
});
