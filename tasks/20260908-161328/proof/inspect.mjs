import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {readFile, writeFile, mkdtemp, mkdir, rm} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {setTimeout as delay} from 'node:timers/promises';
const work=fileURLToPath(new URL('./restyled/',import.meta.url));
const root=new URL('../comic-opening-poc/',import.meta.url).href;
await mkdir(work,{recursive:true});
const profile=await mkdtemp('/tmp/nova-story-preview-');
const browser=spawn('chromium',['--headless','--no-sandbox','--disable-dev-shm-usage','--disable-gpu','--no-first-run','--no-default-browser-check','--remote-debugging-port=0',`--user-data-dir=${profile}`,'about:blank'],{stdio:['ignore','ignore','pipe']});
await writeFile(`${work}/browser.pid`,String(browser.pid));
let log='',socket;
browser.stderr.on('data',chunk=>log+=chunk);
try {
 let port;
 for(let i=0;i<100;i++){try{port=(await readFile(`${profile}/DevToolsActivePort`,'utf8')).split('\n')[0];break}catch{await delay(100)}}
 assert(port);
 const tabs=await(await fetch(`http://127.0.0.1:${port}/json/list`)).json();
 socket=new WebSocket(tabs.find(t=>t.type==='page').webSocketDebuggerUrl);
 await new Promise(resolve=>socket.addEventListener('open',resolve,{once:true}));
 let id=0;const pending=new Map(),errors=[],requests=[];
 socket.addEventListener('message',event=>{const m=JSON.parse(event.data);if(m.method==='Runtime.exceptionThrown')errors.push(m.params.exceptionDetails);if(m.method==='Network.requestWillBeSent')requests.push(m.params.request.url);if(pending.has(m.id)){const {resolve,reject}=pending.get(m.id);pending.delete(m.id);m.error?reject(m.error):resolve(m.result)}});
 const send=(method,params={})=>new Promise((resolve,reject)=>{const next=++id;pending.set(next,{resolve,reject});socket.send(JSON.stringify({id:next,method,params}))});
 const evaluate=async expression=>{const r=await send('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});if(r.exceptionDetails)throw Error(JSON.stringify(r.exceptionDetails));return r.result.value};
 const navigate=async file=>{await send('Page.navigate',{url:root+file});for(let i=0;i<100;i++){if(await evaluate(`location.href===${JSON.stringify(root+file)}&&document.readyState==='complete'`))break;await delay(60)}await evaluate('document.fonts.ready');await delay(100)};
 const shot=async name=>{const r=await send('Page.captureScreenshot',{format:'png'});await writeFile(`${work}/${name}.png`,Buffer.from(r.data,'base64'))};
 await send('Page.enable');await send('Runtime.enable');await send('Network.enable');await send('Page.bringToFront');
 await send('Emulation.setDeviceMetricsOverride',{width:1500,height:1000,deviceScaleFactor:1,mobile:false});
 for(let n=1;n<=4;n++){
  await navigate(`page-0${n}.svg`);
  assert.equal(await evaluate("document.querySelectorAll('.title-card').length"),n===1||n===4?1:0);
  const overflow=await evaluate(`(()=>{const bad=[];for(const box of document.querySelectorAll('.dialogue,.title-card')){const frame=box.querySelector('path,rect').getBBox();for(const t of box.querySelectorAll('text')){const b=t.getBBox();if(b.x<frame.x+10||b.x+b.width>frame.x+frame.width-10||b.y<frame.y||b.y+b.height>frame.y+frame.height)bad.push(t.textContent)}}return bad})()`);
  assert.deepEqual(overflow,[],`Page ${n} lettering fits its boxes`);
  await shot(`page-0${n}`);
 }
 const reports=[];
 for(const width of [1440,390]){
  await send('Emulation.setDeviceMetricsOverride',{width,height:1100,deviceScaleFactor:1,mobile:false});
  await navigate('index.html');
  const hrefs=await evaluate("[...document.querySelectorAll('a[href]')].map(a=>a.getAttribute('href')).filter(href=>!href.startsWith('#'))");
  for(const href of hrefs){const target=new URL(href,root);assert.equal(target.protocol,'file:');await readFile(target)}
  for(let n=1;n<=4;n++){
   await evaluate(`document.querySelector('[data-number="${n}"]').click()`);await delay(100);await evaluate('scrollTo(0,0)');
   assert.equal(await evaluate("document.querySelectorAll('.page:not([hidden])').length"),1);
   assert.equal(await evaluate("document.querySelector('[aria-current=page]').dataset.number"),String(n));
   assert(await evaluate('document.documentElement.scrollWidth<=innerWidth'));
   assert.equal(await evaluate("document.querySelectorAll('svg').length"),4);
   await shot(`${width}-page-${n}`);
   reports.push({width,page:n});
  }
  assert(await evaluate("document.getElementById('next').disabled"));
  await evaluate("document.getElementById('previous').click()");await delay(80);
  assert.equal(await evaluate("document.querySelector('[aria-current=page]').dataset.number"),'3');
  await evaluate("document.querySelector('[data-number=\"1\"]').click()");await delay(80);
  assert(await evaluate("document.getElementById('previous').disabled"));
  await send('Page.bringToFront');await evaluate("document.activeElement.blur()");
  await send('Input.dispatchKeyEvent',{type:'keyDown',key:'ArrowRight',code:'ArrowRight',windowsVirtualKeyCode:39});
  await send('Input.dispatchKeyEvent',{type:'keyUp',key:'ArrowRight',code:'ArrowRight',windowsVirtualKeyCode:39});await delay(80);
  assert.equal(await evaluate("document.querySelector('[aria-current=page]').dataset.number"),'2');
  await evaluate("document.getElementById('art').click()");
  assert.equal(await evaluate("getComputedStyle(document.querySelector('#page-2 .dialogue')).visibility"),'hidden');
  assert(await evaluate("[...document.querySelectorAll('.title-card,.scene-art')].every(n=>getComputedStyle(n).visibility==='visible')"));
  await shot(`${width}-art-only`);
  await evaluate("document.getElementById('art').click();document.querySelector('#page-2 details').open=true");
  assert.equal(await evaluate("getComputedStyle(document.querySelector('#page-2 .dialogue')).visibility"),'visible');
  if(width===390){await evaluate("document.querySelector('#page-2 details').scrollIntoView()");await shot('390-transcript')}
  await evaluate("document.getElementById('contact').click();scrollTo(0,0)");
  assert.equal(await evaluate("document.querySelectorAll('.page:not([hidden])').length"),4);
  assert(await evaluate('document.documentElement.scrollWidth<=innerWidth'));
  await shot(`${width}-contact-sheet`);
 }
 assert.equal(errors.length,0,JSON.stringify(errors));
 assert(requests.length>0&&requests.every(url=>url.startsWith('file:')),JSON.stringify(requests));
 await writeFile(`${work}/browser-checks.json`,JSON.stringify({reports,errors,requests,checks:['direct file loading','raw SVGs','page navigation','keyboard','art toggle','contact sheet','mobile transcripts','no overflow','local comparison links','cards and artwork remain visible under Art only']},null,2));
 console.log('Four SVG pages and eight page/viewport views inspected. Navigation, keyboard, art toggle, contact sheet and transcripts pass. No script exceptions.');
}finally{
 socket?.close();browser.kill('SIGTERM');await writeFile(`${work}/browser.log`,log);
 for(let i=0;i<50&&browser.exitCode===null&&browser.signalCode===null;i++)await delay(100);
 if(browser.exitCode!==null||browser.signalCode!==null)await rm(profile,{recursive:true,force:true});
}
