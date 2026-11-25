import { open, test } from "nsh/test";

const OUT = "../../.visual-shots";

test("capture desktop light screens", async () => {
  const app = await open("..", {
    width: 1240,
    height: 800,
    colorScheme: "light",
  });

  await app.waitFor("最近更新");
  await app.screenshot(`${OUT}/01-discover-light.png`);

  await app.press("搜索");
  await app.waitFor("热搜");
  await app.screenshot(`${OUT}/02-search-light.png`);

  await app.press("追剧");
  await app.waitFor("我的追剧");
  await app.screenshot(`${OUT}/03-watchlist-light.png`);

  await app.press("历史");
  await app.waitFor("最近观看");
  await app.screenshot(`${OUT}/04-history-light.png`);

  await app.press("离线");
  await app.waitFor("已下载");
  await app.screenshot(`${OUT}/05-offline-light.png`);

  await app.press("设置");
  await app.waitFor("外观");
  await app.screenshot(`${OUT}/06-settings-light.png`);

  await app.press("发现");
  await app.waitFor("TOP 1");
  await app.press("TOP 1");
  await app.waitFor("离线下载");
  await app.screenshot(`${OUT}/07-player-light.png`);
});

test("capture desktop dark screens", async () => {
  const app = await open("..", {
    width: 1240,
    height: 800,
    colorScheme: "dark",
  });

  await app.waitFor("最近更新");
  await app.screenshot(`${OUT}/08-discover-dark.png`);

  await app.press("设置");
  await app.waitFor("外观");
  await app.screenshot(`${OUT}/09-settings-dark.png`);
});

test("capture narrow width layout", async () => {
  const app = await open("..", {
    width: 390,
    height: 844,
    colorScheme: "light",
  });

  await app.waitFor("最近更新");
  await app.screenshot(`${OUT}/10-discover-narrow.png`);
});
