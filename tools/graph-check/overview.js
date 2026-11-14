// Follow the reader's existing "About the hashes compared" route to the
// recorded identities. Accept a generated file or a deployed overview URL.
// npm install playwright; node overview.js INDEX.html|URL [CHROMIUM]
const assert = require('node:assert/strict');
const fs = require('node:fs/promises');
const path = require('node:path');
const { pathToFileURL, fileURLToPath } = require('node:url');
const { chromium } = require('playwright');

async function headers(url) {
  const text = url.startsWith('file:')
    ? await fs.readFile(fileURLToPath(url), 'utf8')
    : await fetch(url, { signal: AbortSignal.timeout(15000) }).then(r => {
      assert.equal(r.status, 200, `samples are reachable: ${url}`);
      return r.text();
    });
  return text.split('\n').filter(l => l.startsWith('# ')).flatMap(l => {
    const match = /^# ([^:]+): (.*)$/.exec(l);
    return match ? [[match[1], match[2]]] : [];
  });
}

function contrast(a, b) {
  const luminance = color => color.match(/[\d.]+/g).slice(0, 3)
    .map(x => +x / 255).map(x => x <= .04045 ? x / 12.92 : ((x + .055) / 1.055) ** 2.4)
    .reduce((sum, x, i) => sum + x * [.2126, .7152, .0722][i], 0);
  const x = luminance(a), y = luminance(b);
  return (Math.max(x, y) + .05) / (Math.min(x, y) + .05);
}

async function checkProvenance(page, tap = false) {
  const act = locator => tap ? locator.tap() : locator.click();
  const summary = page.getByText('About the hashes compared', { exact: true });
  assert.equal(await summary.count(), 1, 'the familiar hashes section is discoverable by its name');
  const door = summary.locator('..');
  assert.equal(await door.evaluate(d => d.tagName), 'DETAILS', 'the existing section remains a disclosure');
  assert.equal(await door.locator(':scope > summary').textContent(), 'About the hashes compared');
  assert.equal(await door.evaluate(d => d.open), false, 'provenance starts collapsed');
  assert.equal(await door.locator('.run-record:visible').count(), 0, 'results readers see no expanded records');
  assert(!/[0-9a-f]{40,64}/.test(await page.locator('.grid thead').innerText()), 'long version/commit strings stay out of result headers');
  await act(door.locator(':scope > summary'));
  const machines = door.locator('details.record');
  assert(await machines.count() > 0, 'the section identifies the machines behind the results');
  let runs = 0, identities = 0;
  for (let m = 0; m < await machines.count(); m++) {
    const machine = machines.nth(m);
    await act(machine.locator(':scope > summary'));
    const records = machine.locator('.run-record');
    assert(await records.count() > 0, 'each machine exposes its recorded identities');
    for (let n = 0; n < await records.count(); n++) {
      const record = records.nth(n);
      const data = await record.evaluate(r => ({
        kind: r.querySelector('h4').textContent,
        rows: [...r.querySelectorAll('dt')].map(t => ({label: t.textContent, value: t.nextElementSibling.textContent, href: t.nextElementSibling.querySelector('a')?.href})),
      }));
      const sample = data.rows.find(r => r.label === 'Samples');
      assert(sample?.href, 'each run links its raw samples');
      const suffix = {'Library run': '/bench-hashes.samples.tsv', 'File-tool run': '/b3sum.samples.tsv', 'Earlier library run': '/before/bench-hashes.samples.tsv'}[data.kind];
      assert(suffix && new URL(sample.href).pathname.endsWith(suffix), 'the record identifies its kind of run');
      if (data.kind === 'Library run') assert(!new URL(sample.href).pathname.includes('/before/'));
      const raw = await headers(sample.href);
      const value = key => raw.find(([k]) => k === key)?.[1];
      assert.equal(data.rows.find(r => r.label === 'bench-hashes')?.value, value('bench-hashes version'), 'the complete measured benchmark version is retained');
      assert.equal(data.rows.find(r => r.label === 'Measured')?.value, value('timestamp'), 'each run keeps its own date');
      const commit = data.rows.find(r => r.label === 'bench-hashes commit');
      assert(commit?.value.includes(value('git commit')), 'the benchmark commit is retained');
      assert(commit?.href.endsWith('/tree/' + value('git commit')), 'the commit link names that recorded build');
      const sources = raw.filter(([k]) => k.startsWith('source ') || k === 'contender');
      assert(sources.length > 0, 'the run records contender identities');
      for (const [key, original] of sources) {
        const expected = key === 'contender' ? original.slice(original.indexOf(': ') + 2) : original;
        assert(data.rows.some(r => r.value === expected), `${data.kind}: preserves ${key}: ${original}`);
        identities++;
      }
      const style = await record.locator('dd').first().evaluate(d => ({
        color: getComputedStyle(d).color, background: getComputedStyle(document.body).backgroundColor,
      }));
      assert(contrast(style.color, style.background) >= 4.5, 'opened provenance has readable text contrast');
      assert(await record.evaluate(r => [...r.querySelectorAll('dd')].every(d => d.getBoundingClientRect().right <= document.documentElement.clientWidth + 1)), 'record values wrap inside the viewport');
      runs++;
    }
    await act(machine.locator(':scope > summary'));
  }
  assert(runs > 0, 'current comparisons expose their run records');
  await act(door.locator(':scope > summary'));
  assert.equal(await door.locator('.run-record:visible').count(), 0, 'closing the section clears the results-reading path');
  return { machines: await machines.count(), runs, identities };
}

async function main() {
  const [file, exe] = process.argv.slice(2);
  const url = /^https?:/.test(file) ? file : pathToFileURL(path.resolve(file)).href;
  const browser = await chromium.launch(exe ? { executablePath: exe, args: ['--no-sandbox'] } : {});
  try {
    for (const mobile of [false, true]) {
      const page = await browser.newPage({viewport: {width: mobile ? 390 : 1280, height: 900}, isMobile: mobile, hasTouch: mobile});
      const errors = [];
      page.on('pageerror', e => errors.push(e.message));
      await page.goto(url);
      console.log(mobile ? 'phone' : 'desktop', await checkProvenance(page, mobile));
      assert.deepEqual(errors, []);
      await page.context().close();
    }
  } finally { await browser.close(); }
}
module.exports = { checkProvenance };
if (require.main === module) main().catch(e => { console.error(e); process.exitCode = 1; });
