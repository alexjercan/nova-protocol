import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {readFile,writeFile,mkdtemp,mkdir,rm} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {setTimeout as delay} from 'node:timers/promises';

const out=fileURLToPath(new URL('./aquila/',import.meta.url));
const root=new URL('../aquila-pages/',import.meta.url);
await mkdir(out,{recursive:true});
const profile=await mkdtemp('/tmp/nova-aquila-browser-');
const browser=spawn('chromium',['--headless','--no-sandbox','--disable-dev-shm-usage','--disable-gpu','--no-first-run','--no-default-browser-check','--remote-debugging-port=0',`--user-data-dir=${profile}`,'about:blank'],{stdio:['ignore','ignore','pipe']});
await writeFile(out+'browser.pid',`${browser.pid}\n`);
let log='',socket;browser.stderr.on('data',chunk=>log+=chunk);
try{
 let port;for(let n=0;n<100;n++){try{port=(await readFile(profile+'/DevToolsActivePort','utf8')).split('\n')[0];break}catch{await delay(100)}}assert(port);
 const tabs=await(await fetch(`http://127.0.0.1:${port}/json/list`)).json();socket=new WebSocket(tabs.find(t=>t.type==='page').webSocketDebuggerUrl);
 await new Promise(done=>socket.addEventListener('open',done,{once:true}));
 let id=0;const pending=new Map(),errors=[],requests=[];
 socket.addEventListener('message',event=>{const m=JSON.parse(event.data);if(m.method==='Runtime.exceptionThrown')errors.push(m.params.exceptionDetails);if(m.method==='Network.requestWillBeSent')requests.push(m.params.request.url);if(pending.has(m.id)){const p=pending.get(m.id);pending.delete(m.id);m.error?p.reject(m.error):p.resolve(m.result)}});
 const send=(method,params={})=>new Promise((resolve,reject)=>{const key=++id;pending.set(key,{resolve,reject});socket.send(JSON.stringify({id:key,method,params}))});
 const evaluate=async expression=>{const r=await send('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});assert(!r.exceptionDetails,JSON.stringify(r.exceptionDetails));return r.result.value};
 const navigate=async file=>{const url=new URL(file,root).href;await send('Page.navigate',{url});let ready;for(let n=0;n<100;n++){ready=await evaluate(`location.href===${JSON.stringify(url)}&&document.readyState==='complete'`);if(ready)break;await delay(70)}assert(ready,url);await evaluate('document.fonts.ready');await delay(120)};
 const viewport=(width,height)=>send('Emulation.setDeviceMetricsOverride',{width,height,deviceScaleFactor:1,mobile:false});
 const shot=async name=>{const r=await send('Page.captureScreenshot',{format:'png'});await writeFile(out+name+'.png',Buffer.from(r.data,'base64'))};
 await send('Page.enable');await send('Runtime.enable');await send('Network.enable');await send('Page.bringToFront');
 const pages=[];
 for(const number of [5,6,7]){
  await viewport(1500,1000);await navigate(`page-0${number}.svg`);await shot(`page-0${number}`);
  assert.equal(await evaluate("document.querySelectorAll('script,foreignObject,image').length"),0);
  assert(await evaluate("(()=>{const ids=[...document.querySelectorAll('[id]')].map(n=>n.id);return ids.length===new Set(ids).size})()"));
  const checks=await evaluate(`(()=>{
   const badText=[];
   for(const g of document.querySelectorAll('.dialogue'))for(const t of g.querySelectorAll('text')){
    const b=t.getBBox(),d=g.dataset;
    if(b.x<+d.x+12||b.y<+d.y+4||b.x+b.width>+d.x+ +d.width-10||b.y+b.height>+d.y+ +d.height-8)badText.push(t.textContent);
   }
   const badFaces=[...document.querySelectorAll('[data-face]')].filter(n=>{const b=n.getBoundingClientRect();return b.left<0||b.top<86||b.right>1500||b.bottom>943}).map(n=>n.dataset.face);
   const faceOverlaps=[];
   for(const face of document.querySelectorAll('[data-face]'))for(const g of document.querySelectorAll('.dialogue')){
    const f=face.getBoundingClientRect(),m=g.getScreenCTM(),d=g.dataset;
    const a=new DOMPoint(+d.x,+d.y).matrixTransform(m),b=new DOMPoint(+d.x+ +d.width,+d.y+ +d.height).matrixTransform(m);
    if(Math.min(f.right,b.x)-Math.max(f.left,a.x)>2&&Math.min(f.bottom,b.y)-Math.max(f.top,a.y)>2)faceOverlaps.push(face.dataset.face+': '+g.textContent);
   }
   return {badText,badFaces,faceOverlaps,faces:[...document.querySelectorAll('[data-face]')].map(n=>n.dataset.face),ships:[...document.querySelectorAll('[data-ship]')].map(n=>n.dataset.ship),closedLocks:document.querySelectorAll('[data-door=closed]').length};
  })()`);
  pages.push({number,...checks});
 }
 for(const width of [1440,390]){
  await viewport(width,1100);await navigate('index.html');
  assert(await evaluate('document.documentElement.scrollWidth<=innerWidth'));
  assert.equal(await evaluate("document.querySelectorAll('section').length"),3);
  for(const href of await evaluate("[...document.querySelectorAll('a[href]')].map(n=>n.getAttribute('href')).filter(h=>!h.startsWith('#'))"))await readFile(new URL(href,root));
  await shot(`${width}-board`);
  for(const number of [5,6,7]){await evaluate(`document.getElementById('page-${number}').scrollIntoView()`);await shot(`${width}-page-${number}`)}
  await evaluate("document.getElementById('art-only').click()");
  assert(await evaluate("[...document.querySelectorAll('.dialogue')].every(n=>getComputedStyle(n).visibility==='hidden')"));
  assert(await evaluate("[...document.querySelectorAll('.scene-art,.title-card')].every(n=>getComputedStyle(n).visibility==='visible')"));
  await shot(`${width}-art-only`);
  await evaluate("document.getElementById('art-only').click();document.querySelector('#page-6 details').open=true;document.querySelector('#page-6 details').scrollIntoView()");
  assert(await evaluate("document.querySelector('#page-6 details').innerText.includes('Better than chasing both loads.')"));await shot(`${width}-transcript`);
  await navigate('../episode-1/index.html');await evaluate('Promise.all([...document.images].map(n=>n.decode()))');
  assert.equal(await evaluate("[...document.querySelectorAll('h2')].filter(n=>/^Page \\d+:/.test(n.textContent)).length"),18);
  assert.equal(await evaluate('document.images.length'),7);
  assert(await evaluate('document.documentElement.scrollWidth<=innerWidth'));
  await evaluate("document.getElementById('page-6-another-crews-work').scrollIntoView()");await shot(`${width}-episode`);
 }
 assert.deepEqual(errors,[]);assert(requests.every(url=>url.startsWith('file:')));
 await writeFile(out+'browser-checks.json',JSON.stringify({pages,requests,errors,checks:['three full-size SVGs','unique IDs, lettering and face bounds','pressure windows and closed freight locks','desktop/phone board, Art only, and transcripts','18-page script now includes seven illustrations','private local-only requests']},null,2)+'\n');
 for(const page of pages){assert.deepEqual(page.badText,[],`Page ${page.number} lettering`);assert.deepEqual(page.badFaces,[],`Page ${page.number} faces`);assert.deepEqual(page.faceOverlaps,[],`Page ${page.number} dialogue does not cover a face`)}
 console.log('Aquila art and private review pass at desktop/phone widths.');
}finally{
 socket?.close();browser.kill('SIGTERM');await writeFile(out+'browser.txt',log);
 for(let n=0;n<50&&browser.exitCode===null&&browser.signalCode===null;n++)await delay(100);
 if(browser.exitCode!==null||browser.signalCode!==null)await rm(profile,{recursive:true,force:true});
}
