import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {readFile,writeFile,mkdir,mkdtemp,rm,copyFile} from 'node:fs/promises';
import {createServer} from 'node:http';
import {fileURLToPath} from 'node:url';
import {createRequire} from 'node:module';
import {setTimeout as delay} from 'node:timers/promises';
import path from 'node:path';
const require=createRequire(import.meta.url);
const repo=fileURLToPath(new URL('../../../',import.meta.url));
const out=fileURLToPath(new URL('./episode-1/',import.meta.url));
const temp=(await readFile('/tmp/nova-episode-review-current','utf8')).trim();
const prefixRoot=path.join(temp,'site/nova-protocol');
const {discoverComics,comicIndexPage,comicSeasonPage,comicReaderPage}=require('../../../web/comic-build.js');
await mkdir(out,{recursive:true});
const server=createServer(async(req,res)=>{
 try{
  const name=decodeURIComponent(new URL(req.url,'http://local').pathname);
  const root=name.startsWith('/nova-protocol/')?path.join(temp,'site'):path.join(repo,'web/dist');
  const target=path.resolve(root,'.'+name+(name.endsWith('/')?'index.html':''));
  assert(target.startsWith(root+'/'));
  const data=await readFile(target);
  const mime={'.html':'text/html','.js':'text/javascript','.svg':'image/svg+xml','.png':'image/png','.css':'text/css'};
  res.writeHead(200,{'Content-Type':mime[path.extname(target)]??'application/octet-stream'});res.end(data);
 }catch{res.writeHead(404);res.end('Not found');}
});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
const base=`http://127.0.0.1:${server.address().port}`;
const profile=await mkdtemp('/tmp/nova-episode-browser-');
const browser=spawn('chromium',['--headless','--no-sandbox','--disable-dev-shm-usage','--disable-gpu','--no-first-run','--no-default-browser-check','--remote-debugging-port=0',`--user-data-dir=${profile}`,'about:blank'],{stdio:['ignore','ignore','pipe']});
await writeFile(out+'browser.pid',String(browser.pid));
let log='',socket;browser.stderr.on('data',chunk=>log+=chunk);
const reports=[];
try{
 let port;for(let i=0;i<100;i++){try{port=(await readFile(profile+'/DevToolsActivePort','utf8')).split('\n')[0];break}catch{await delay(100)}}assert(port);
 const tabs=await(await fetch(`http://127.0.0.1:${port}/json/list`)).json();
 socket=new WebSocket(tabs.find(t=>t.type==='page').webSocketDebuggerUrl);
 await new Promise(resolve=>socket.addEventListener('open',resolve,{once:true}));
 let id=0;const pending=new Map(),errors=[];
 socket.addEventListener('message',event=>{const m=JSON.parse(event.data);if(m.method==='Runtime.exceptionThrown')errors.push(m.params.exceptionDetails);if(pending.has(m.id)){const p=pending.get(m.id);pending.delete(m.id);m.error?p.reject(m.error):p.resolve(m.result)}});
 const send=(method,params={})=>new Promise((resolve,reject)=>{const key=++id;pending.set(key,{resolve,reject});socket.send(JSON.stringify({id:key,method,params}))});
 const evaluate=async expression=>{const r=await send('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});if(r.exceptionDetails)throw Error(JSON.stringify(r.exceptionDetails));return r.result.value};
 const navigate=async url=>{await send('Page.navigate',{url});for(let i=0;i<100;i++){if(await evaluate(`location.href===${JSON.stringify(url)}&&document.readyState==='complete'`))break;await delay(60)}assert(await evaluate(`location.href===${JSON.stringify(url)}&&document.readyState==='complete'`));await evaluate('document.fonts.ready');await delay(250)};
 const shot=async name=>{const data=await send('Page.captureScreenshot',{format:'png'});await writeFile(out+name+'.png',Buffer.from(data.data,'base64'))};
 const fits=async()=>assert(await evaluate('document.documentElement.scrollWidth<=innerWidth'),'No horizontal overflow');
 await send('Page.enable');await send('Runtime.enable');await send('Page.bringToFront');
 for(const prefix of ['/','/nova-protocol/'])for(const width of [1440,390]){
  await send('Emulation.setDeviceMetricsOverride',{width,height:1000,deviceScaleFactor:1,mobile:false});
  for(const [route,label] of [['story/','archive'],['story/demo/','demo-contents'],['story/demo/episode-1/','reader']]){
   await navigate(base+prefix+route);await fits();
   if(label==='archive')assert(await evaluate('document.body.textContent.includes("No story episodes have been released")'));
   if(label==='reader'){
    assert.equal(await evaluate('document.querySelectorAll(".comic-page:not([hidden])").length'),1);
    assert(await evaluate('document.querySelector("[data-page-next]").disabled'));
    assert(await evaluate('document.getElementById("comic-contents").hidden'));
    await evaluate('document.querySelector(".story-page-text").open=true');
    assert(await evaluate('document.querySelector("[data-page-transcript]").textContent.includes("Interface only")'));
    assert.equal((await fetch(await evaluate('document.querySelector("[data-page-art]").href'))).status,200);
   }
   await shot(`${prefix==='/'?'root':'prefix'}-${width}-${label}`);
   reports.push({prefix,width,route});
  }
 }
 const demo=discoverComics()[0];
 const ep=(id,pages)=>({...demo.episodes[0],id,title:id,pages});
 const page=(id,text)=>({...demo.episodes[0].pages[0],id,title:id,transcript:text});
 const season={...demo,kind:'season',path:'season-1',sequence:1,title:'Fixture season',summary:'Browser fixture, never published.',episodes:[ep('episode-1',[page('one','First page. <b>Literal text</b>.'),page('two','Second page.')]),ep('episode-2',[page('one','Next episode.')])]};
 await mkdir(prefixRoot+'/assets/story/season-1',{recursive:true});
 await copyFile(prefixRoot+'/assets/story/demo/cover.svg',prefixRoot+'/assets/story/season-1/cover.svg');
 await copyFile(path.join(repo,'tasks/20260908-161328/comic-opening-poc/page-01.svg'),prefixRoot+'/assets/story/season-1/fixture-page.svg');
 const plugins=[comicIndexPage([season],'/nova-protocol/'),comicSeasonPage(season,'/nova-protocol/'),...season.episodes.map(e=>comicReaderPage(season,e,'/nova-protocol/'))];
 for(const plugin of plugins){
  const target=path.join(prefixRoot,plugin.userOptions.filename);await mkdir(path.dirname(target),{recursive:true});
  // Reuse the compiled demo module in this temporary route fixture, not public story content.
  const html=plugin.userOptions.templateContent.replace('"path":"season-1"','"path":"demo"').replace('</body>','<script defer src="/nova-protocol/story.js"></script></body>');
  await writeFile(target,html);
 }
 for(const width of [1440,390]){
  await send('Emulation.setDeviceMetricsOverride',{width,height:1000,deviceScaleFactor:1,mobile:false});
  for(const route of ['story/','story/season-1/']){await navigate(base+'/nova-protocol/'+route);await fits();assert(await evaluate("[...document.querySelectorAll('.post-card__title')].every(n=>{const b=n.getBoundingClientRect(),c=n.closest('.post-card').getBoundingClientRect();return b.width>10&&b.left>=c.left&&b.right<=c.right&&b.top>=c.top&&b.bottom<=c.bottom})"),'Card titles remain inside their cards');await shot(`fixture-${width}-${route==='story/'?'archive':'season'}`)}
  await navigate(base+'/nova-protocol/story/season-1/episode-1/#two');
  assert.equal(await evaluate('document.querySelector("[data-page-current]").textContent'),'02');
  assert.equal(await evaluate('document.querySelector("[data-page-transcript]").textContent'),'Second page.');
  await evaluate('document.querySelector("[data-page-previous]").click()');
  assert.equal(await evaluate('document.querySelector("[data-page-current]").textContent'),'01');
  assert.equal(await evaluate('document.querySelector("[data-page-transcript] b")'),null);
  await evaluate('document.querySelector("[data-contents-toggle]").click()');
  assert.equal(await evaluate('document.getElementById("comic-contents").hidden'),false);
  await evaluate('document.querySelector("[data-page-link=two]").click()');
  assert.equal(await evaluate('document.querySelector("[data-page-current]").textContent'),'02');
  if(width===390)assert(await evaluate('document.getElementById("comic-contents").hidden'));
  await evaluate('document.querySelector("[data-page-viewport]").focus()');
  await send('Input.dispatchKeyEvent',{type:'keyDown',key:'ArrowLeft',code:'ArrowLeft',windowsVirtualKeyCode:37});
  await send('Input.dispatchKeyEvent',{type:'keyUp',key:'ArrowLeft',code:'ArrowLeft',windowsVirtualKeyCode:37});
  assert.equal(await evaluate('document.querySelector("[data-page-current]").textContent'),'01');
  assert(await evaluate('document.querySelector("[data-page-viewport]").dispatchEvent(new WheelEvent("wheel",{deltaY:120,ctrlKey:true,cancelable:true}))'));
  assert.equal(await evaluate('document.querySelector("[data-page-current]").textContent'),'01');
  await evaluate('document.querySelector("[data-page-viewport]").dispatchEvent(new WheelEvent("wheel",{deltaY:120,cancelable:true}))');
  assert.equal(await evaluate('document.querySelector("[data-page-current]").textContent'),'02');
  // The renderer unit test verifies this exact artPage DOM; inspect its CSS with the accepted full-color SVG.
  await evaluate(`(()=>{const p=document.getElementById('one');p.className='comic-page comic-art-page';const img=document.createElement('img');img.src=${JSON.stringify(base+'/nova-protocol/assets/story/season-1/fixture-page.svg')};img.alt='Accepted page used only in this CSS fixture';p.replaceChildren(img);document.querySelector('[data-page-previous]').click();document.querySelector('.story-page-text').open=true;if(!document.getElementById('comic-contents').hidden)document.querySelector('[data-contents-toggle]').click()})()`);
  await evaluate('Promise.all([...document.images].map(i=>i.decode()))');await delay(250);
  assert.equal(await evaluate('getComputedStyle(document.getElementById("one"),"::after").content'),'none');
  assert.equal(await evaluate('getComputedStyle(document.querySelector("#one img")).objectFit'),'contain');
  assert.equal(await evaluate('getComputedStyle(document.querySelector("#one img")).filter'),'none');
  await fits();await shot(`fixture-${width}-art-reader`);
  const next=await evaluate('[...document.querySelectorAll(".story-episode-links a")].find(a=>a.textContent.includes("Next episode")).href');
  await navigate(next);assert(await evaluate('document.body.textContent.includes("Previous episode")'));
 }
 await send('Emulation.setDeviceMetricsOverride',{width:1200,height:1000,deviceScaleFactor:1,mobile:false});
 await navigate(new URL('../episode-1/index.html',import.meta.url).href);
 assert.equal(await evaluate('[...document.querySelectorAll("h2")].filter(h=>/^Page \\d+:/.test(h.textContent)).length'),18);
 assert.equal(await evaluate('document.querySelectorAll(".retained img").length'),4);
 await shot('script-review');
 await evaluate('document.getElementById("page-10-the-decision").scrollIntoView()');await shot('script-decision');
 assert.deepEqual(errors,[]);
 await writeFile(out+'browser-checks.json',JSON.stringify({reports,errors,checks:['public root and prefix archives/contents/readers','demo not presented as released story','temporary season and two-episode route fixture','fragment loading, buttons, contents, keyboard and wheel','Ctrl-wheel not consumed','literal transcripts','full-color artPage CSS: contain, no filter, no scanline overlay','next episode links','private 18-page script including four retained pages'],fixture_scope:'Only the temporary prefixed build is populated with fixture episodes; production web/dist and public sources remain demo-only.'},null,2)+'\n');
 console.log('Public root/prefix views, temporary episode fixtures, art-page layout and private script inspected');
}finally{
 socket?.close();browser.kill('SIGTERM');await writeFile(out+'browser.log',log);
 for(let i=0;i<50&&browser.exitCode===null&&browser.signalCode===null;i++)await delay(100);
 if(browser.exitCode!==null||browser.signalCode!==null)await rm(profile,{recursive:true,force:true});
 server.closeAllConnections();await new Promise(resolve=>server.close(resolve));
}
