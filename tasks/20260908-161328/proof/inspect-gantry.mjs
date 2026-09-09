import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {readFile,writeFile,mkdtemp,mkdir,rm} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
import {createServer} from 'node:http';
import {resolve,extname,sep} from 'node:path';
import {setTimeout as delay} from 'node:timers/promises';

const out=fileURLToPath(new URL('./gantry/',import.meta.url));
const root=new URL('../gantry-study/',import.meta.url);
await mkdir(out,{recursive:true});
let server,origin;
if(process.argv[2]){
 const directory=resolve(process.argv[2]);
 server=createServer(async(req,res)=>{try{
  let name=decodeURIComponent(new URL(req.url,'http://local').pathname);if(name.endsWith('/'))name+='index.html';
  const target=resolve(directory,'.'+name);assert(target.startsWith(directory+sep));
  const data=await readFile(target),mime={'.html':'text/html','.js':'text/javascript','.css':'text/css','.svg':'image/svg+xml','.woff2':'font/woff2','.json':'application/json'};
  res.writeHead(200,{'Content-Type':mime[extname(target)]??'application/octet-stream'});res.end(data);
 }catch{res.writeHead(404);res.end('Not found')}});
 await new Promise(done=>server.listen(0,'127.0.0.1',done));origin=`http://127.0.0.1:${server.address().port}`;
}
const profile=await mkdtemp('/tmp/nova-gantry-browser-');
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
 const images=[['gantry-conditions.svg',1500,1100],['crew-comic.svg',1500,1000],['crew-lore.svg',1500,1000],['../../../web/src/assets/lore/gantry-design-concept.svg',1500,1100],...['nadia-sen','owen-park','ivo-marin'].map(n=>[`../../../web/src/assets/lore/${n}-portrait-concept.svg`,700,840])];
 const geometries=[];
 for(const [file,width,height] of images){
  await viewport(width,height);await navigate(file);
  assert.equal(await evaluate("document.querySelectorAll('script,foreignObject,image').length"),0);
  assert(await evaluate("(()=>{const ids=[...document.querySelectorAll('[id]')].map(n=>n.id);return ids.length===new Set(ids).size})()"));
  assert(await evaluate("[...document.querySelectorAll('text,[data-face]')].every(n=>{const b=n.getBoundingClientRect();return b.left>=0&&b.top>=0&&b.right<=innerWidth&&b.bottom<=innerHeight})"));
  if(file.startsWith('crew-')){
   assert.deepEqual(await evaluate("[...document.querySelectorAll('[data-face]')].map(n=>n.dataset.face)"),['nadia','owen','ivo']);
   geometries.push(await evaluate("[...document.querySelectorAll('[data-character]')].map(n=>n.outerHTML)"));
   assert.equal(await evaluate("document.querySelectorAll('filter').length"),file==='crew-lore.svg'?3:0);
  }
  if(file==='gantry-conditions.svg')assert.equal(await evaluate("document.querySelectorAll('[data-state=stranded]').length"),2);
  if(file.includes('assets/lore/'))assert.equal(await evaluate("document.querySelectorAll('[data-state]').length"),0);
  await shot(file.split('/').at(-1).replace('.svg',''));
 }
 assert.deepEqual(geometries[0],geometries[1]);
 for(const width of [1440,390]){
  await viewport(width,1100);await navigate('index.html');await evaluate('Promise.all([...document.images].map(n=>n.decode()))');
  assert(await evaluate('document.documentElement.scrollWidth<=innerWidth'));
  for(const href of await evaluate("[...document.querySelectorAll('a[href]')].map(n=>n.getAttribute('href'))"))await readFile(new URL(href,root));
  await shot(`${width}-review`);
  await evaluate("document.querySelectorAll('section')[1].scrollIntoView()");await shot(`${width}-crew`);
 }
 const privateRequests=[...requests];assert(privateRequests.every(url=>url.startsWith('file:')));
 const publicViews=[];
 if(origin)for(const width of [1440,390]){
  await viewport(width,1100);
  for(const slug of ['ships/gantry','characters/nadia-sen','characters/owen-park','characters/ivo-marin']){
   await navigate(`${origin}/lore/${slug}/`);await evaluate('Promise.all([...document.images].map(n=>n.decode()))');
   assert(await evaluate('document.documentElement.scrollWidth<=innerWidth'));
   assert(await evaluate("[...document.querySelectorAll('article img')].every(n=>n.complete&&n.naturalWidth>0)"));
   await evaluate("document.querySelector('article img').scrollIntoView({block:'center',behavior:'instant'})");await delay(150);
   assert(await evaluate("document.querySelector('article img').getBoundingClientRect().top<innerHeight/2"));
   await shot(`site-${width}-${slug.replace('/','-')}`);publicViews.push({width,slug});
  }
 }
 assert.deepEqual(errors,[]);
 assert(requests.every(url=>url.startsWith('file:')||(origin&&url.startsWith(origin+'/'))||url.startsWith('https://fonts.googleapis.com/')||url.startsWith('https://fonts.gstatic.com/')));
 await writeFile(out+'browser-checks.json',JSON.stringify({images:images.map(i=>i[0]),privateRequests,publicViews,errors,checks:['seven standalone SVGs','text and face bounds','distinct frontal crew identities','comic/lore crew geometry identical','damage present only in private comparison','local-only private review','desktop/phone review and public articles']},null,2)+'\n');
 console.log(`Seven SVGs and private board inspected; ${publicViews.length} public article views passed.`);
}finally{
 server?.closeAllConnections();server?.close();socket?.close();browser.kill('SIGTERM');await writeFile(out+'browser.log',log);
 for(let n=0;n<50&&browser.exitCode===null&&browser.signalCode===null;n++)await delay(100);
 if(browser.exitCode!==null||browser.signalCode!==null)await rm(profile,{recursive:true,force:true});
}
