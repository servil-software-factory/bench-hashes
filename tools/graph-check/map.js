// Drives a generated bench-hashes.map.html in Chromium: no script error;
// every chart opens in place and closes; its row and column headers light
// while it is open; a header greys its charts; every call name links to its
// documentation; a link to one chart (#solo|one|lent) opens it; on a touch
// screen a tap shows the values at its point, the chart open, and the
// button puts it back.
// npm install playwright; node map.js MAP.html [CHROMIUM]
const path = require('path');
const assert = require('assert');
const { pathToFileURL } = require('url');
const { chromium } = require('playwright');
async function main() {
  const [file, exe] = process.argv.slice(2);
  const url = pathToFileURL(path.resolve(file)).href;
  const browser = await chromium.launch(exe ? { executablePath: exe, args: ['--no-sandbox'] } : {});
  const page = await browser.newPage({ viewport: { width: 1200, height: 900 } });
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.goto(url);
  const charts = await page.locator('.cell:not(.empty)').count();
  assert(charts >= 1, 'some chart');
  const docs = await page.$$eval('a.doc', as => as.map(a => a.href));
  assert.equal(docs.length, charts, 'every chart names its call, a link');
  for (const d of docs) assert(/^https:\/\//.test(d), d);
  for (let i = 0; i < charts; i++) {
    const cell = page.locator('.cell:not(.empty)').nth(i);
    await cell.scrollIntoViewIfNeeded();
    await cell.click({ position: { x: 100, y: 30 } });
    await page.waitForTimeout(260);
    assert.equal(await page.locator('.zoom').count(), 1, `chart ${i} opens`);
    assert.equal(await page.locator('.h.lit').count(), 2, `chart ${i}: its row and column light`);
    const box = await page.locator('.zoom').boundingBox();
    // Inside the opened chart and the window both, clear of its title.
    await page.mouse.move(box.x + box.width / 2, Math.min(box.y + box.height / 2, Math.max(box.y + 60, 850)));
    assert(await page.locator('#tip').isVisible(), `chart ${i}: the values under the pointer`);
    assert(/\d/.test(await page.locator('#tip').textContent()), `chart ${i}: numbers in the tip`);
    const tipBox = await page.locator('#tip').boundingBox(), view = page.viewportSize();
    assert(tipBox.x >= 0 && tipBox.x + tipBox.width <= view.width, `chart ${i}: the values fit inside the window`);
    // A click on the plot (the names above it are toggles) puts it back.
    await page.locator('.zoom').click({ position: { x: box.width / 2, y: box.height * 0.7 } });
    await page.waitForTimeout(300);
    assert.equal(await page.locator('.zoom').count(), 0, `chart ${i} closes`);
    assert.equal(await page.locator('.h.lit').count(), 0, `chart ${i}: headers unlit`);
  }
  await page.locator('.h').first().click();
  assert(await page.locator('.h.show .means').first().isVisible(), "a header's click shows what it means");
  await page.locator('.h').first().click();
  await page.mouse.move(0, 0);
  assert(!(await page.locator('.h .means').first().isVisible()), 'a second click hides it');
  await page.locator('.h').nth(1).hover();
  assert(await page.locator('.h').nth(1).locator('.means').isVisible(), 'pointing at a header shows what it means');
  await page.mouse.move(0, 0);
  const first = await page.locator('.cell:not(.empty)').first().getAttribute('data-key');
  await page.goto(url + '#' + encodeURIComponent(first.split(':').slice(1).join(':')));
  await page.waitForTimeout(300);
  assert.equal(await page.locator('.zoom').count(), 1, 'a link to one chart opens it');
  // Escape puts the chart the link opened back.
  await page.keyboard.press('Escape');
  await page.waitForTimeout(300);
  assert.equal(await page.locator('.zoom').count(), 0, 'Escape puts the chart back');
  // What the lines measure: CPU time adds dotted lines where the run recorded
  // it; turning wall-clock time off leaves those alone; energy waits.
  const solid = async () => page.locator('.cell polyline:not([stroke-dasharray])').count();
  const dotted = async () => page.locator('.cell polyline[stroke-dasharray^="0.1"]').count();
  const wallOnly = await solid();
  assert(wallOnly > 0 && (await dotted()) === 0, 'wall-clock time alone at first');
  const hasCpu = await page.evaluate(() => SECTIONS.some(S => Object.values(S.cells).some(c => c.cpu.some(Boolean))));
  await page.locator('.measures button', { hasText: 'CPU' }).first().click();
  if (hasCpu) assert((await dotted()) > 0, 'CPU time draws dotted lines');
  await page.locator('.measures button', { hasText: 'wall-clock' }).first().click();
  assert.equal(await solid(), 0, 'wall-clock time off: no solid lines');
  // Energy: its chip works where the run counted it (dashed lines), and is
  // disabled where it counted none.
  const hasEnergy = await page.evaluate(() => SECTIONS.some(S => Object.values(S.cells).some(c => c.energy.some(Boolean))));
  const energyChip = page.locator('.measures button', { hasText: 'joule' }).first();
  assert.equal(await energyChip.isDisabled(), !hasEnergy, 'the energy chip follows the data');
  if (hasEnergy) {
    await energyChip.click();
    assert((await page.locator('.cell polyline[stroke-dasharray="5 3"]').count()) > 0, 'energy draws dashed lines');
    await page.locator('.measures button', { hasText: 'joule' }).first().click();
  }
  await page.locator('.measures button', { hasText: 'wall-clock' }).first().click();
  await page.locator('.measures button', { hasText: 'CPU' }).first().click();
  assert.equal(await solid(), wallOnly, 'back to wall-clock time alone');
  // Cost per byte: the same lines, the axis in ns per byte, lower is faster.
  await page.locator('.measures .switch').click();
  assert.equal(await solid(), wallOnly, 'cost view draws the same lines');
  await page.locator('.cell:not(.empty)').first().click(); await page.waitForTimeout(300);
  assert(/ns per byte.*lower is better/.test(await page.locator('.zoom').innerHTML()), 'the cost axis names its unit and direction');
  await page.keyboard.press('Escape'); await page.waitForTimeout(300);
  await page.locator('.measures .switch').click();
  // A click outside an opened chart puts it back.
  await page.locator('.cell:not(.empty)').first().click(); await page.waitForTimeout(300);
  await page.mouse.click(5, 5); await page.waitForTimeout(300);
  assert.equal(await page.locator('.zoom').count(), 0, 'a click outside puts the chart back');
  // The widest values (every measure shown, a narrower window) still fit
  // inside the window.
  const narrowPage = await browser.newPage({ viewport: { width: 1000, height: 800 } });
  narrowPage.on('pageerror', e => errors.push(e.message));
  await narrowPage.goto(url);
  for (const chip of ['CPU', 'joule']) {
    const b = narrowPage.locator('.measures button', { hasText: chip }).first();
    if (!(await b.isDisabled())) await b.click();
  }
  await narrowPage.locator('.cell:not(.empty)').first().click(); await narrowPage.waitForTimeout(300);
  const wide = await narrowPage.locator('.zoom').boundingBox();
  await narrowPage.mouse.move(wide.x + wide.width * 0.9, Math.min(wide.y + wide.height * 0.6, 760));
  const wideTip = await narrowPage.locator('#tip').boundingBox();
  assert(wideTip.x >= 0 && wideTip.x + wideTip.width <= 1000 && wideTip.y >= 0 && wideTip.y + wideTip.height <= 800, 'every measure\'s values fit inside the window');
  await narrowPage.close();
  const phone = await browser.newPage({ viewport: { width: 420, height: 860 }, hasTouch: true, isMobile: true });
  phone.on('pageerror', e => errors.push(e.message));
  await phone.goto(url);
  await checkPhoneTargets(phone);
  // A name in the open chart is its contender's toggle: a tap hides its
  // lines (fewer drawn), marks the name off, and the chart stays open; a
  // second tap brings them back.
  // A contender's marks: a line as a polyline or a path, a lone point as a circle.
  const lines = async () => phone.locator('.zoom svg polyline, .zoom svg path, .zoom svg circle').count();
  const all = await lines();
  const shown = phone.locator('.zoom .key button:not(.off)').first();
  const name = await shown.textContent();
  const named = () => phone.locator('.zoom .key button', { hasText: new RegExp('^' + name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&') + '$') });
  await shown.tap(); await phone.waitForTimeout(300);
  assert.equal(await phone.locator('.zoom').count(), 1, 'a name tapped keeps the chart open');
  assert(/\boff\b/.test(await named().getAttribute('class')), 'the tapped name shows off');
  assert((await lines()) < all, 'its lines are hidden');
  await named().tap(); await phone.waitForTimeout(300);
  assert.equal(await lines(), all, 'a second tap brings them back');
  await phone.context().close();
  assert.deepEqual(errors, []);
  console.log(`${charts} charts opened and closed, ${docs.length} documentation links, headers, headers' meanings, a chart's link, Escape, what the lines measure, touch, names as toggles: pass`);
  await browser.close();
}

// A fresh touch-enabled map page. Leaves its first chart open for further checks.
// This DOM-only contract is shared with historical maps' older data/legend forms.
async function checkPhoneTargets(phone) {
  await phone.locator('.cell:not(.empty)').first().tap();
  await phone.waitForTimeout(300);
  assert.equal(await phone.locator('.zoom').count(), 1, 'a phone tap opens its chart');
  assert(await phone.locator('.zoom .title a.doc').evaluate(a => getComputedStyle(a).pointerEvents !== 'none'), 'the open chart keeps its documentation link active');
  const zoom = await phone.locator('.zoom').boundingBox();
  await phone.touchscreen.tap(zoom.x + zoom.width * 0.6, zoom.y + zoom.height * 0.5);
  await phone.waitForTimeout(150);
  assert.equal(await phone.locator('.zoom').count(), 1, 'a tap on an open chart keeps it open');
  assert(await phone.locator('#tip').isVisible(), 'a tap shows the values at its point');
  await phone.locator('.zoom .close').tap();
  await phone.waitForTimeout(300);
  assert.equal(await phone.locator('.zoom').count(), 0, 'the button puts the chart back');
  // On a phone the small chart's call name is part of its opening target.
  const small = phone.locator('.cell:not(.empty)').first();
  await small.scrollIntoViewIfNeeded();
  const c = await small.boundingBox(), d = await small.locator('a.doc').boundingBox();
  const x = (Math.max(c.x, d.x) + Math.min(c.x+c.width, d.x+d.width)) / 2;
  const y = (Math.max(c.y, d.y) + Math.min(c.y+c.height, d.y+d.height)) / 2;
  await phone.touchscreen.tap(x, y);
  await phone.waitForTimeout(300);
  assert.equal(await phone.locator('.zoom').count(), 1, 'a tap on the small call name opens its chart');
}

module.exports = { checkPhoneTargets };
if (require.main === module) main().catch(e => { console.error(e); process.exit(1); });
