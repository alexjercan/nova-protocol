import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {readFile, writeFile, mkdtemp, mkdir, rm} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {createServer} from 'node:http';
import {resolve, extname, sep} from 'node:path';
import {setTimeout as delay} from 'node:timers/promises';

const work = fileURLToPath(new URL('./clean-line/', import.meta.url));
const root = new URL('../clean-line-study/', import.meta.url).href;
const studies = ['01-close-up', '02-window', '03-exterior'];
const publicViews = [];
let server, publicOrigin;
if (process.argv[2]) {
  const directory = resolve(process.argv[2]);
  const mime = {'.html':'text/html','.js':'text/javascript','.css':'text/css','.svg':'image/svg+xml','.png':'image/png','.webp':'image/webp','.woff2':'font/woff2','.woff':'font/woff','.json':'application/json'};
  server = createServer(async (request, response) => {
    try {
      let pathname = decodeURIComponent(new URL(request.url, 'http://local').pathname);
      if (pathname.endsWith('/')) pathname += 'index.html';
      const target = resolve(directory, '.' + pathname);
      if (!target.startsWith(directory + sep)) throw Error('Outside build');
      const content = await readFile(target);
      response.writeHead(200, {'Content-Type':mime[extname(target)] ?? 'application/octet-stream'});
      response.end(content);
    } catch { response.writeHead(404); response.end('Not found'); }
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  publicOrigin = `http://127.0.0.1:${server.address().port}`;
}
await mkdir(work, {recursive: true});
const profile = await mkdtemp('/tmp/nova-clean-line-browser-');
const browser = spawn('chromium', [
  '--headless', '--no-sandbox', '--disable-dev-shm-usage', '--disable-gpu',
  '--no-first-run', '--no-default-browser-check', '--remote-debugging-port=0',
  `--user-data-dir=${profile}`, 'about:blank',
], {stdio: ['ignore', 'ignore', 'pipe']});
await writeFile(`${work}/browser.pid`, `${browser.pid}\n`);
let log = '', socket;
browser.stderr.on('data', chunk => log += chunk);

try {
  let port;
  for (let i = 0; i < 100; i++) {
    try {
      port = (await readFile(`${profile}/DevToolsActivePort`, 'utf8')).split('\n')[0];
      break;
    } catch { await delay(100); }
  }
  assert(port, 'Chromium exposes its owned debugging port');
  const tabs = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  socket = new WebSocket(tabs.find(t => t.type === 'page').webSocketDebuggerUrl);
  await new Promise(resolve => socket.addEventListener('open', resolve, {once: true}));
  let id = 0;
  const pending = new Map(), errors = [], requests = [];
  socket.addEventListener('message', event => {
    const message = JSON.parse(event.data);
    if (message.method === 'Runtime.exceptionThrown') errors.push(message.params.exceptionDetails);
    if (message.method === 'Network.requestWillBeSent') requests.push(message.params.request.url);
    if (pending.has(message.id)) {
      const {resolve, reject} = pending.get(message.id);
      pending.delete(message.id);
      message.error ? reject(message.error) : resolve(message.result);
    }
  });
  const send = (method, params = {}) => new Promise((resolve, reject) => {
    const next = ++id;
    pending.set(next, {resolve, reject});
    socket.send(JSON.stringify({id: next, method, params}));
  });
  const evaluate = async expression => {
    const result = await send('Runtime.evaluate', {expression, returnByValue: true, awaitPromise: true});
    assert(!result.exceptionDetails, JSON.stringify(result.exceptionDetails));
    return result.result.value;
  };
  const navigate = async file => {
    const url = new URL(file, root).href;
    await send('Page.navigate', {url});
    let ready = false;
    for (let i = 0; i < 100; i++) {
      ready = await evaluate(`location.href===${JSON.stringify(url)}&&document.readyState==='complete'`);
      if (ready) break;
      await delay(60);
    }
    assert(ready, `${file} loaded`);
    await evaluate('document.fonts.ready');
    await delay(120);
  };
  const shot = async name => {
    const result = await send('Page.captureScreenshot', {format: 'png'});
    await writeFile(`${work}/${name}.png`, Buffer.from(result.data, 'base64'));
  };
  await send('Page.enable');
  await send('Page.bringToFront');
  await send('Runtime.enable');
  await send('Network.enable');
  await send('Emulation.setDeviceMetricsOverride', {width: 1500, height: 1000, deviceScaleFactor: 1, mobile: false});
  for (const study of studies) {
    await navigate(`${study}.svg`);
    const overflow = await evaluate(`(() => {
      const bad=[];
      for(const box of document.querySelectorAll('.speech')) {
        const {x,y,width,height}=box.dataset;
        for(const t of box.querySelectorAll('text')) {
          const b=t.getBBox();
          if(b.x<Number(x)+16||b.x+b.width>Number(x)+Number(width)-16||b.y<Number(y)+10||b.y+b.height>Number(y)+Number(height)-10) bad.push(t.textContent);
        }
      }
      return bad;
    })()`);
    assert.deepEqual(overflow, [], `${study} lettering fits its balloon`);
    assert.equal(await evaluate("document.querySelectorAll('script,foreignObject').length"), 0);
    assert.equal(await evaluate("document.querySelectorAll('filter').length"), 1);
    await shot(study);
  }
  const views = [];
  for (const width of [1440, 390]) {
    await send('Emulation.setDeviceMetricsOverride', {width, height: 1100, deviceScaleFactor: 1, mobile: false});
    await navigate('index.html');
    assert.equal(await evaluate("document.querySelectorAll('section>svg').length"), 3);
    assert(await evaluate('document.documentElement.scrollWidth<=innerWidth'));
    assert(await evaluate("(()=>{const ids=[...document.querySelectorAll('[id]')].map(n=>n.id);return ids.length===new Set(ids).size})()"));
    for (const study of studies) {
      await evaluate(`document.querySelector('nav a[href="#${study}"]').click()`);
      await delay(120);
      assert.equal(await evaluate('location.hash'), `#${study}`);
      await shot(`${width}-${study}`);
      views.push({width, study});
    }
    const geometry = await evaluate("[...document.querySelectorAll('.scene-art')].map(n=>n.innerHTML)");
    await evaluate("document.getElementById('scheme').click();document.getElementById('01-close-up').scrollIntoView()");
    assert.deepEqual(await evaluate("[...document.querySelectorAll('.scene-art')].map(n=>n.innerHTML)"), geometry);
    assert(await evaluate("[...document.querySelectorAll('.scene-art')].every(n=>document.getElementById(n.getAttribute('filter').slice(5,-1)))"));
    assert(await evaluate("[...document.querySelectorAll('.speech')].every(n=>getComputedStyle(n).filter==='none')"));
    await shot(`${width}-lore-colors`);
    await evaluate("document.getElementById('scheme').click()");
    assert(await evaluate("[...document.querySelectorAll('.scene-art')].every(n=>!n.hasAttribute('filter'))"));
    await evaluate("document.getElementById('art').click()");
    assert.equal(await evaluate("getComputedStyle(document.querySelector('.speech')).visibility"), 'hidden');
    await evaluate("document.getElementById('01-close-up').scrollIntoView()");
    await shot(`${width}-art-only`);
    await evaluate("document.getElementById('art').focus()");
    assert.equal(await evaluate('document.activeElement.id'), 'art');
    await send('Input.dispatchKeyEvent', {type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, text: '\r'});
    await send('Input.dispatchKeyEvent', {type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13});
    await delay(120);
    assert.equal(await evaluate("getComputedStyle(document.querySelector('.speech')).visibility"), 'visible');
    await evaluate("document.getElementById('01-close-up').querySelector('details').open=true;document.getElementById('01-close-up').querySelector('details').scrollIntoView()");
    assert(await evaluate('document.documentElement.scrollWidth<=innerWidth'));
    await shot(`${width}-transcript`);
  }
  for (const name of ['kaveri-design-concept','ebro-design-concept','elena-ward-portrait-concept']) {
    await send('Emulation.setDeviceMetricsOverride', {width: name.startsWith('elena')?700:1500, height: name.startsWith('elena')?840:1100, deviceScaleFactor: 1, mobile: false});
    await navigate(`../../../web/src/assets/lore/${name}.svg`);
    await shot(name);
  }
  const privateRequests = [...requests];
  assert(privateRequests.every(url => url.startsWith('file:')), JSON.stringify(privateRequests));
  if (publicOrigin) {
    for (const width of [1440,390]) {
      await send('Emulation.setDeviceMetricsOverride', {width,height:1100,deviceScaleFactor:1,mobile:false});
      for (const slug of ['ships/kaveri','ships/ebro','characters/elena-ward']) {
        await navigate(`${publicOrigin}/nova-protocol/lore/${slug}/`);
        await evaluate("Promise.all([...document.images].map(img=>img.decode().catch(()=>{})))");
        assert(await evaluate('document.documentElement.scrollWidth<=innerWidth'));
        assert(await evaluate("[...document.images].every(img=>img.complete&&img.naturalWidth>0)"));
        const images = await evaluate("[...document.images].map(img=>img.currentSrc)");
        assert(images.some(src=>src.includes(slug.split('/').at(-1))&&src.endsWith('.svg')));
        if (width===390) {
          await evaluate("document.querySelector('article img').scrollIntoView({block:'center',behavior:'instant'})");
          await delay(150);
          assert(await evaluate("document.querySelector('article img').getBoundingClientRect().top<innerHeight/2"));
        }
        await shot(`site-${width}-${slug.replace('/','-')}`);
        publicViews.push({width,slug});
      }
    }
  }
  assert.deepEqual(errors, []);
  const fontOrigins = ['https://fonts.googleapis.com/', 'https://fonts.gstatic.com/'];
  assert(requests.every(url => url.startsWith('file:') || (publicOrigin && (url.startsWith(publicOrigin+'/') || fontOrigins.some(origin=>url.startsWith(origin))))), JSON.stringify(requests));
  await writeFile(`${work}/browser-checks.json`, JSON.stringify({views, publicViews, errors, privateRequests, requests, checks: [
    'Three standalone SVGs', 'Lettering bounds', 'Six section/viewport views',
    'Unique inline IDs', 'Anchor navigation', 'Art-only toggle and keyboard activation',
    'Text descriptions and dialogue', 'No horizontal overflow', 'No network dependencies',
    'Color-only lore preview with unchanged geometry and separate lettering', 'Three standalone lore exports',
  ]}, null, 2) + '\n');
  console.log(`Three SVGs, six private section/viewport views, and ${publicViews.length} built article views passed; lettering, color-only styling, navigation, keyboard, and transcripts checked.`);
} finally {
  server?.closeAllConnections();
  server?.close();
  socket?.close();
  browser.kill('SIGTERM');
  await writeFile(`${work}/browser.log`, log);
  for (let i = 0; i < 50 && browser.exitCode === null && browser.signalCode === null; i++) await delay(100);
  if (browser.exitCode !== null || browser.signalCode !== null) await rm(profile, {recursive: true, force: true});
}
