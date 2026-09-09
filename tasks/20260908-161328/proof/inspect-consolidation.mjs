import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {readFile,writeFile,mkdtemp,mkdir,rm} from 'node:fs/promises';
import {fileURLToPath,pathToFileURL} from 'node:url';
import {createServer} from 'node:http';
import {resolve,extname,sep} from 'node:path';
import {setTimeout as delay} from 'node:timers/promises';

const out=fileURLToPath(new URL(process.env.STORY_TEST_OUT||'./consolidation/',import.meta.url));await mkdir(out,{recursive:true});
const base=process.env.STORY_TEST_PREFIX||'';
const directory=resolve(process.argv[2]||'web/.cache/story/development/site');
let server,origin=process.argv[3];
if(!origin){server=createServer(async(req,res)=>{try{let name=decodeURIComponent(new URL(req.url,'http://local').pathname);if(base&&name.startsWith(base+'/'))name=name.slice(base.length);
if(name==='/__review/wrap/'){
const html=await readFile(resolve(directory,'story/season-1/episode-1/index.html'),'utf8');res.writeHead(200,{'Content-Type':'text/html'});res.end(html.replace(/"text":("(?:[^"\\]|\\.)*")/,(_,value)=>'"text":'+JSON.stringify(JSON.parse(value).replace(/\n/g,' '))));return}
const unsafe=name.match(/^\/__review\/unsafe-(script|event|url)\/$/);
if(unsafe){const html=await readFile(resolve(directory,'story/season-1/episode-1/index.html'),'utf8');res.writeHead(200,{'Content-Type':'text/html'});res.end(html.replace(/"image"\s*:\s*"episode-1\/page-01.svg"/,`"image":"malicious-${unsafe[1]}.svg"`));return}
const bad=name.match(/\/malicious-(script|event|url)\.svg$/);
if(bad){const payload={script:'<script>window.__bad=true</script>',event:'<rect onload="window.__bad=true"/>',url:'<rect fill="url(https://invalid.example/paint.svg)"/>'};res.writeHead(200,{'Content-Type':'image/svg+xml'});res.end(`<svg xmlns="http://www.w3.org/2000/svg">${payload[bad[1]]}</svg>`);return}
if(name.endsWith('/'))name+='index.html';const target=resolve(directory,'.'+name);assert(target.startsWith(directory+sep));const data=await readFile(target),mime={'.html':'text/html','.js':'text/javascript','.css':'text/css','.svg':'image/svg+xml','.woff2':'font/woff2','.json':'application/json'};res.writeHead(200,{'Content-Type':mime[extname(target)]??'application/octet-stream'});res.end(data)}catch{res.writeHead(404);res.end('Not found')}});await new Promise(done=>server.listen(0,'127.0.0.1',done));origin=`http://127.0.0.1:${server.address().port}`;}
const profile=await mkdtemp('/tmp/nova-reader-browser-');
const browser=spawn('chromium',['--headless','--no-sandbox','--disable-dev-shm-usage','--disable-gpu','--no-first-run','--no-default-browser-check','--remote-debugging-port=0',`--user-data-dir=${profile}`,'about:blank'],{stdio:['ignore','ignore','pipe']});await writeFile(out+'browser.pid',`${browser.pid}\n`);
let log='',socket;browser.stderr.on('data',c=>log+=c);
try{
 let port;for(let n=0;n<100;n++){try{port=(await readFile(profile+'/DevToolsActivePort','utf8')).split('\n')[0];break}catch{await delay(100)}}assert(port);
 const tabs=await(await fetch(`http://127.0.0.1:${port}/json/list`)).json();socket=new WebSocket(tabs.find(t=>t.type==='page').webSocketDebuggerUrl);await new Promise(done=>socket.addEventListener('open',done,{once:true}));
 let id=0;const pending=new Map(),errors=[],requests=[];
 socket.addEventListener('message',event=>{const m=JSON.parse(event.data);if(m.method==='Runtime.exceptionThrown')errors.push(m.params.exceptionDetails);if(m.method==='Network.requestWillBeSent')requests.push(m.params.request.url);if(pending.has(m.id)){const p=pending.get(m.id);pending.delete(m.id);m.error?p.reject(m.error):p.resolve(m.result)}});
 const send=(method,params={})=>new Promise((resolve,reject)=>{const key=++id;pending.set(key,{resolve,reject});socket.send(JSON.stringify({id:key,method,params}))});
 const evaluate=async expression=>{const r=await send('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});assert(!r.exceptionDetails,JSON.stringify(r.exceptionDetails));return r.result.value};
 const wait=async expr=>{for(let n=0;n<160;n++){if(await evaluate(expr))return;await delay(75)}throw new Error(`Timed out: ${expr}`)};
 const navigate=async route=>{const url=origin+base+route;await send('Page.navigate',{url});await wait(`location.href===${JSON.stringify(url)}&&document.readyState==='complete'`);await evaluate('document.fonts.ready');await delay(200)};
 const viewport=(width,height)=>send('Emulation.setDeviceMetricsOverride',{width,height,deviceScaleFactor:1,mobile:false});
 const shot=async name=>{const r=await send('Page.captureScreenshot',{format:'png'});await writeFile(out+name+'.png',Buffer.from(r.data,'base64'))};
 await send('Page.enable');await send('Runtime.enable');await send('Network.enable');await send('Page.bringToFront');
 if(process.env.STORY_PUBLIC_CHECK==='1'){
  for(const width of [1440,390]){
   await viewport(width,1000);
   for(const [name,route] of [['library','/story/']]){
    await navigate(route);await evaluate('Promise.all([...document.images].map(n=>n.decode()))');
    assert(await evaluate('document.documentElement.scrollWidth<=innerWidth'));
    assert(!await evaluate("!!document.querySelector('[data-layout-toggle], [data-art-toggle], .story-draft-label')"));
    if(name==='library')assert(await evaluate("document.body.textContent.includes('No story episodes have been released')"));
    if(name==='reader')await wait("document.querySelectorAll('.comic-page').length===1");
    await shot(`${width}-${name}`);
   }
  }
  assert.deepEqual(errors,[]);await writeFile(out+'browser-checks.json',JSON.stringify({publicOnly:true,base,errors,requests},null,2)+'\n');
  console.log('Public released-only empty archive passed at desktop/phone widths.');
 }else{
 const reports=[];
 for(const width of [1440,390]){
  await viewport(width,1100);
  for(const route of ['/story/','/story/season-1/']){
   await navigate(route);await evaluate('Promise.all([...document.images].map(n=>n.decode()))');assert(await evaluate('document.documentElement.scrollWidth<=innerWidth'));
   assert.equal(await evaluate("document.querySelectorAll('.story-episode').length"),1);assert(await evaluate("document.body.innerText.toLowerCase().includes('7 preview pages')"));assert(!await evaluate("document.body.innerText.includes('Latest episode')"));await shot(`${width}-${route==='/story/'?'library':'season'}`);
  }
  await viewport(width,1000);await navigate('/story/season-1/episode-1/#page-5');
  await wait("document.querySelectorAll('[data-layout-ready=true]').length===7");
  assert.equal(await evaluate("document.querySelector('.comic-page:not([hidden])').id"),'page-5');
  assert(await evaluate("(()=>{const ids=[...document.querySelectorAll('[id]')].map(n=>n.id);return new Set(ids).size===ids.length})()"),'Inline pages have unique IDs');
  for(const number of [1,2,3,4,5,6,7]){
   await evaluate(`location.hash='page-${number}'`);await delay(140);
   const result=await evaluate(`(()=>{const p=document.querySelector('.comic-page:not([hidden])');return {id:p.id,issues:JSON.parse(p.dataset.layoutIssues||'[]'),text:document.querySelector('[data-page-transcript]').textContent,export:document.querySelector('[data-page-art]').href,lettering:p.querySelectorAll('[data-balloon]').length}})()`);
   reports.push({width,...result});await shot(`${width}-page-${number}`);
   assert.equal(result.id,`page-${number}`);assert(result.text.trim());assert(result.export.startsWith('blob:'));
   if(width===1440){const svg=await evaluate(`fetch(${JSON.stringify(result.export)}).then(r=>r.text())`);await writeFile(out+`export-page-${number}.svg`,svg);}
  }
  await evaluate("document.querySelector('[data-layout-toggle]').click()");await shot(`${width}-checks`);
  assert(await evaluate("!document.querySelector('.comic-page:not([hidden]) .comic-layout-report').hidden"));
  await evaluate("document.querySelector('[data-layout-toggle]').click();document.querySelector('[data-art-toggle]').click()");
  assert.equal(await evaluate("getComputedStyle(document.querySelector('.comic-page:not([hidden]) .comic-lettering')).visibility"),'hidden');await shot(`${width}-art-only`);
  await evaluate("document.querySelector('[data-art-toggle]').click();document.querySelector('[data-contents-toggle]').click();document.querySelector('[data-page-link=page-3]').click()");await delay(100);
  assert.equal(await evaluate("document.querySelector('.comic-page:not([hidden])').id"),'page-3');
  if(width===390)assert(await evaluate("document.getElementById('comic-contents').hidden"));
  await evaluate("document.querySelector('[data-page-viewport]').focus()");await send('Input.dispatchKeyEvent',{type:'keyDown',key:'ArrowRight',code:'ArrowRight'});await send('Input.dispatchKeyEvent',{type:'keyUp',key:'ArrowRight',code:'ArrowRight'});await delay(150);assert.equal(await evaluate('location.hash'),'#page-4');
  await send('Page.reload',{ignoreCache:true});await delay(400);await wait("document.querySelectorAll('[data-layout-ready=true]').length===7&&document.querySelector('.comic-page:not([hidden])')?.id==='page-4'");
  await evaluate("document.querySelector('.story-page-text').open=true");await shot(`${width}-transcript`);assert(await evaluate('document.documentElement.scrollWidth<=innerWidth'));
 }
 let paragraphWrap=false;
 if(server){
  await navigate('/__review/wrap/');await wait("document.querySelectorAll('[data-layout-ready=true]').length===7");
  const wrapped=await evaluate("[...document.querySelectorAll('#page-1 [data-balloon=\"0\"] text')].slice(1).map(t=>t.textContent)");
  assert(wrapped.length>1);assert.equal(wrapped.join(' '),"Coffee's still hot. Take a minute before somebody finds you another job.");
  assert.deepEqual(await evaluate("JSON.parse(document.querySelector('#page-1').dataset.layoutIssues||'[]')"),[]);paragraphWrap=true;
 }
 const rejectedScenes=[];
 if(server)for(const kind of ['script','event','url']){
  await navigate(`/__review/unsafe-${kind}/`);
  await wait("!!document.querySelector('#page-1')?.dataset.layoutError");
  assert.equal(await evaluate('window.__bad'),undefined);
  await evaluate("document.querySelector('.story-page-text').open=true");
  assert(await evaluate("document.querySelector('[data-page-transcript]').textContent.includes('Coffee')"));
  rejectedScenes.push(kind);
 }
 let watchCheck=false;
 if(process.env.STORY_WATCH_CHECK==='1'){
  const source=new URL('../../../web/src/comics/season-1/episode-1/opening.py',import.meta.url);
  const original=await readFile(source,'utf8');assert(original.includes("Coffee's still hot. Take a minute"));
  try{
   await writeFile(source,original.replace("Coffee's still hot. Take a minute","Coffee's still hot. Take a moment"));
   await wait("document.querySelector('#page-1 .comic-lettering')?.textContent.includes('Take a moment')");
   assert.equal(await evaluate('location.hash'),'#page-4');
  }finally{await writeFile(source,original)}
  await wait("document.querySelector('#page-1 .comic-lettering')?.textContent.includes('Take a minute')");
  assert.equal(await evaluate('location.hash'),'#page-4');watchCheck=true;
 }
 const exports=[];
 await viewport(1500,1000);
 for(const number of [1,2,3,4,5,6,7]){
  const original=new URL(`../${number<=4?'comic-opening-poc':'aquila-pages'}/page-0${number}.svg`,import.meta.url).href;
  await send('Page.navigate',{url:original});await wait(`location.href===${JSON.stringify(original)}&&document.readyState==='complete'`);await evaluate('document.fonts.ready');
  const before=(await send('Page.captureScreenshot',{format:'png'})).data;
  const exported=pathToFileURL(out+`export-page-${number}.svg`).href;
  await send('Page.navigate',{url:exported});await wait(`location.href===${JSON.stringify(exported)}&&document.readyState==='complete'`);await evaluate('document.fonts.ready');
  const after=(await send('Page.captureScreenshot',{format:'png'})).data;await writeFile(out+`export-page-${number}.png`,Buffer.from(after,'base64'));
  const difference=await evaluate(`(async()=>{const sources=${JSON.stringify([before,after])};const pixels=[];for(const source of sources){const image=new Image();image.src='data:image/png;base64,'+source;await image.decode();const canvas=new OffscreenCanvas(1500,1000);const c=canvas.getContext('2d');c.drawImage(image,0,0);pixels.push(c.getImageData(0,0,1500,1000).data)}let changed=0;for(let i=0;i<pixels[0].length;i+=4)if([0,1,2,3].some(k=>pixels[0][i+k]!==pixels[1][i+k]))changed++;return changed})()`);
  exports.push({number,changedPixels:difference});
 }
 await writeFile(out+'browser-checks.json',JSON.stringify({reports,exports,watchCheck,paragraphWrap,rejectedScenes,errors,requests},null,2)+'\n');assert.deepEqual(errors,[]);
 for(const report of reports)assert.deepEqual(report.issues,[],`Layout ${report.width}/${report.id}`);
 console.log('Series library, season contents, seven reader pages, diagnostics, exports, reload and navigation passed at desktop/phone widths.');
 }
}finally{server?.closeAllConnections();server?.close();socket?.close();browser.kill('SIGTERM');await writeFile(out+'browser.txt',log);for(let n=0;n<50&&browser.exitCode===null&&browser.signalCode===null;n++)await delay(100);if(browser.exitCode!==null||browser.signalCode!==null)await rm(profile,{recursive:true,force:true})}
