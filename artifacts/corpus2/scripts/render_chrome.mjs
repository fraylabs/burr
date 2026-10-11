// Inspect an actual Burr server with installed Google Chrome. Requires Playwright.
// argv: URL, evidence prefix, Playwright module path. Hold the shared lock while
// loading a cold model; screenshots and Chrome logs stay in local evidence.
import fs from 'node:fs/promises';
const [url, prefix, modulePath] = process.argv.slice(2);
const { chromium } = await import(modulePath);
const browser = await chromium.launch({
  executablePath: '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  headless: true,
});
const page = await browser.newPage({viewport: {width: 1600, height: 1100}});
const errors = [];
page.on('pageerror', error => errors.push(String(error)));
page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
try {
  const response = await page.goto(url, {waitUntil: 'domcontentloaded', timeout: 600000});
  await page.waitForFunction(() => typeof burrMeshes !== 'undefined' &&
    burrMeshes.length === burrManifest.definitions.length, {timeout: 600000});
  await page.evaluate(() => window.postMessage({type: 'burr:set-render-mode', mode: 'solid'}, '*'));
  await page.waitForTimeout(1200);
  await page.screenshot({path: `${prefix}.isometric.png`});
  for (const view of ['front', 'top', 'right']) {
    await page.evaluate(view => setPresetView(view), view);
    await page.waitForTimeout(500);
    await page.screenshot({path: `${prefix}.${view}.png`});
  }
  const geometry = await page.evaluate(() => ({
    definitions: burrManifest.definitions.length,
    occurrences: burrManifest.occurrences.length,
    renderer: gl.getParameter(gl.RENDERER),
    webglError: gl.getError(),
    canvas: [canvas.width,canvas.height],
  }));
  await fs.writeFile(`${prefix}.chrome.json`, JSON.stringify({url, httpStatus: response.status(), chromeVersion: browser.version(), geometry, errors}, null, 2));
} catch (error) {
  errors.push(String(error));
  await page.screenshot({path: `${prefix}.error.png`});
  await fs.writeFile(`${prefix}.chrome.json`, JSON.stringify({url, chromeVersion: browser.version(), errors}, null, 2));
  throw error;
} finally {
  await browser.close();
}
