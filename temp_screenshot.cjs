const { chromium } = require('playwright');
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  await page.goto('http://127.0.0.1:3000');
  await page.waitForTimeout(1000);
  await page.click('.nav-button[aria-label="Настройки"]');
  await page.waitForTimeout(1000);
  await page.screenshot({ path: 'C:/Users/artyo/.gemini/antigravity/brain/da15fa2a-70b1-42f0-9b81-eb370cf29fd8/settings_screenshot.png' });
  await browser.close();
  console.log('Done');
})();
