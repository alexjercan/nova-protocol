// Browser proof through CDP pipes and intercepted files. No HTTP server or listening port.
import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {readFile,writeFile,mkdir,mkdtemp,rm} from 'node:fs/promises';
import {resolve,extname,sep} from 'node:path';
import {setTimeout as delay} from 'node:timers/promises';
const root=resolve('.'),directory=resolve(process.argv[2]||'web/.cache/story/navigation/local');
const out=resolve('tasks/20260908-161328/proof/navigation',process.env.PROOF_NAME||'browser');await mkdir(out,{recursive:true});
const base=process.env.PUBLIC_PATH||'/';const origin='https://nova-review.invalid';
const profile=await mkdtemp('/tmp/nova-navigation-browser-');
const browser=spawn('chromium',['--headless','--no-sandbox','--disable-dev-shm-usage','--disable-gpu','--no-first-run','--no-default-browser-check','--remote-debugging-pipe',`--user-data-dir=${profile}`],{stdio:['ignore','ignore','pipe','pipe','pipe']});
await writeFile(out+'/browser.pid',`${browser.pid}\n`);
let log='',buffer='',counter=0,session;const pending=new Map(),errors=[];
browser.stderr.on('data',b=>log+=b);
function send(method,params={},sessionId=session){return new Promise((resolve,reject)=>{const id=++counter;pending.set(id,{resolve,reject});browser.stdio[3].write(JSON.stringify({id,method,params,...(sessionId?{sessionId}:{})})+'\0');});}
async function intercept(event){
 const request=event.params;let name=new URL(request.request.url).pathname;let type='text/plain',body='';
 try{
 if(!request.request.url.startsWith(origin)){type='text/css';body='';}
 else {
  assert(name.startsWith(base));name=name.slice(base.length-1);
  const evil=name.match(/evil-(script|event|url)\.svg$/);
  if(evil){type='image/svg+xml';body=`<svg xmlns="http://www.w3.org/2000/svg">${{script:'<script>window.__bad=true</script>',event:'<rect onload="window.__bad=true"/>',url:'<rect fill="url(https://outside.invalid/a.svg)"/>'}[evil[1]]}</svg>`;}
  else if(name.startsWith('/__golden/')){type='image/svg+xml';body=await readFile(resolve(root,'tasks/20260908-161328/proof/consolidation',name.split('/').pop()));}
  else {
   const test=name.match(/^\/__(wrap|unsafe-(script|event|url))\/$/);
   if(test)name='/story/season-1/episode-1/';
   if(name.endsWith('/'))name+='index.html';
   const file=resolve(directory,'.'+name);assert(file.startsWith(directory+sep));body=await readFile(file);
   type={'.html':'text/html','.js':'text/javascript','.css':'text/css','.svg':'image/svg+xml','.json':'application/json','.png':'image/png','.woff2':'font/woff2'}[extname(file)]||'application/octet-stream';
   if(test){body=body.toString().replace(/(<script[^>]*id="comic-definition"[^>]*>)([\s\S]*?)(<\/script>)/,(_,a,json,b)=>{const manifest=JSON.parse(json);if(test[1]==='wrap')manifest.pages[0].definition.panels[1].lettering['rina-1'].breakAfter=[];else manifest.pages[0].definition.panels[0].image=`evil-${test[2]}.svg`;return a+JSON.stringify(manifest).replace(/</g,'\\u003c')+b;});}
  }
 }
 await send('Fetch.fulfillRequest',{requestId:request.requestId,responseCode:200,responseHeaders:[{name:'Content-Type',value:type}],body:Buffer.from(body).toString('base64')},event.sessionId);
 }catch(error){errors.push({request:request.request.url,error:String(error)});await send('Fetch.fulfillRequest',{requestId:request.requestId,responseCode:404,body:''},event.sessionId);}
}
browser.stdio[4].on('data',chunk=>{buffer+=chunk;let end;while((end=buffer.indexOf('\0'))>=0){const m=JSON.parse(buffer.slice(0,end));buffer=buffer.slice(end+1);if(pending.has(m.id)){const p=pending.get(m.id);pending.delete(m.id);m.error?p.reject(m.error):p.resolve(m.result);}else if(m.method==='Fetch.requestPaused')void intercept(m);else if(m.method==='Runtime.exceptionThrown')errors.push(m.params.exceptionDetails);}});
const evaluate=async expression=>{const result=await send('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});assert(!result.exceptionDetails,JSON.stringify(result.exceptionDetails));return result.result.value;};
const wait=async expr=>{for(let i=0;i<180;i++){if(await evaluate(expr))return;await delay(75);}throw new Error('Timed out: '+expr);};
const navigate=async route=>{const url=origin+base+route.replace(/^\//,'');await send('Page.navigate',{url});await wait(`location.href===${JSON.stringify(url)} && document.readyState==='complete'`);await delay(250);};
const viewport=width=>send('Emulation.setDeviceMetricsOverride',{width,height:1000,deviceScaleFactor:1,mobile:false});
const shot=async name=>{const r=await send('Page.captureScreenshot',{format:'png'});await writeFile(`${out}/${name}.png`,Buffer.from(r.data,'base64'));};
try{
 const {targetId}=await send('Target.createTarget',{url:'about:blank'},null);session=(await send('Target.attachToTarget',{targetId,flatten:true},null)).sessionId;
 await send('Page.enable');await send('Runtime.enable');await send('Fetch.enable',{patterns:[{urlPattern:'*'}]});
 const reports=[],pixels=[];
 for(const width of [1440,390]){
  await viewport(width);await navigate('/story/');assert(await evaluate('document.documentElement.scrollWidth<=innerWidth'));await shot(`${width}-library`);
  if(process.env.PUBLIC_ONLY==='1'){assert(await evaluate("document.body.textContent.includes('No story episodes have been released')"));continue;}
  assert.equal(await evaluate("document.querySelector('h1').textContent"),'Story');
  assert.equal(await evaluate("document.querySelectorAll('main a[href*=\\\"/episode-1/\\\"]').length"),1);
  assert.equal(await evaluate("document.querySelectorAll('main img').length"),1);
  assert(await evaluate("document.querySelector('.story-episode').textContent.includes('7 of 18 pages illustrated')"));
  assert(!await evaluate("/Preview episode|Start reading|Latest episode|Season contents|Read >/.test(document.querySelector('main').textContent)"));
  assert(!await evaluate("!!document.querySelector('.story-season__cover, .story-actions, .story-archive')"));
  await evaluate("(()=>{const original=document.querySelector('.story-season');for(let i=2;i<=8;i++){const section=original.cloneNode(true),heading=section.querySelector('h2');heading.id='fixture-season-'+i;heading.textContent='Fixture season '+i;section.setAttribute('aria-labelledby',heading.id);original.parentElement.append(section);}location.hash='fixture-season-3';})()");
  await wait("(()=>{const top=document.getElementById('fixture-season-3').getBoundingClientRect().top,header=document.querySelector('.site-header').getBoundingClientRect().bottom;return top>=header && top<=header+25})()");
  await shot(`${width}-grouped-fixture`);
  await evaluate("document.querySelector('.story-episode').focus()");
  await send('Input.dispatchKeyEvent',{type:'keyDown',key:'Enter',code:'Enter',windowsVirtualKeyCode:13});await send('Input.dispatchKeyEvent',{type:'keyUp',key:'Enter',code:'Enter',windowsVirtualKeyCode:13});
  await wait("location.pathname.endsWith('/story/season-1/episode-1/') && document.querySelectorAll('[data-layout-ready=true]').length===7");
  assert.equal(await evaluate("document.querySelector('.comic-reader__back').textContent"),'Story');
  assert.equal(await evaluate("document.querySelector('[data-contents-toggle]').textContent"),'Pages');
  await evaluate("document.querySelector('[data-contents-toggle]').click()");
  assert(!await evaluate("document.querySelector('#comic-contents').hidden"));
  await evaluate("document.querySelector('[data-contents-toggle]').click();document.querySelector('.comic-reader__back').click()");
  await wait("location.pathname.endsWith('/story/') && location.hash==='#season-1' && !!document.querySelector('.story-episode')");
  await send('Page.navigate',{url:origin+base+'story/season-1/'});
  await wait("location.pathname.endsWith('/story/') && location.hash==='#season-1' && document.readyState==='complete'");
  await shot(`${width}-season-redirect`);
  await navigate('/story/season-1/episode-1/#page-4');await wait("document.querySelectorAll('[data-layout-ready=true]').length===7");
  await shot(`${width}-initial`);
  assert.equal(await evaluate("document.querySelector('.comic-page:not([hidden])').id"),'page-4');
  assert(await evaluate("(()=>{const ids=[...document.querySelectorAll('[id]')].map(n=>n.id);return new Set(ids).size===ids.length})()"));
  for(let n=1;n<=7;n++){
   await evaluate(`location.hash='page-${n}'`);await delay(150);
   const result=await evaluate("(()=>{const p=document.querySelector('.comic-page:not([hidden])');return {id:p.id,issues:JSON.parse(p.dataset.layoutIssues),panels:JSON.parse(p.dataset.panelExports),image:p.dataset.artUrl,text:document.querySelector('[data-page-transcript]').textContent}})()");
   assert.equal(result.id,`page-${n}`);assert.equal(await evaluate("document.querySelectorAll('.comic-page:not([hidden])').length"),1);assert.deepEqual(result.issues,[],`${width}/${n}`);assert(result.text.trim());reports.push({width,...result});
   assert.equal(await evaluate("document.querySelectorAll('[data-panel-image=art]').length"),result.panels.length);
   for(const p of result.panels){assert((await evaluate(`fetch(${JSON.stringify(p.art)}).then(r=>r.text())`)).includes('<svg'));const native=await evaluate(`fetch(${JSON.stringify(p.composed)}).then(r=>r.text())`);assert(native.includes('data-panel'));if(width===1440)await writeFile(`${out}/panel-${p.id}.svg`,native);}
   if(width===1440){const svg=await evaluate(`fetch(${JSON.stringify(result.image)}).then(r=>r.text())`);await writeFile(`${out}/export-page-${n}.svg`,svg);
    const diff=await evaluate(`(async()=>{const sources=[${JSON.stringify(origin+base+'__golden/export-page-'+n+'.svg')},${JSON.stringify(result.image)}];const data=[];for(const src of sources){const img=new Image();img.src=src;await img.decode();const canvas=new OffscreenCanvas(1500,1000),ctx=canvas.getContext('2d');ctx.drawImage(img,0,0);data.push(ctx.getImageData(0,0,1500,1000).data);}let pixels=0,max=0,minX=1500,minY=1000,maxX=0,maxY=0;for(let i=0;i<data[0].length;i+=4){let d=0;for(let c=0;c<4;c++)d=Math.max(d,Math.abs(data[0][i+c]-data[1][i+c]));if(d){pixels++;max=Math.max(max,d);const x=(i/4)%1500,y=Math.floor(i/4/1500);minX=Math.min(minX,x);maxX=Math.max(maxX,x);minY=Math.min(minY,y);maxY=Math.max(maxY,y);}}return {pixels,max,bounds:[minX,minY,maxX,maxY]};})()`);
    pixels.push({page:n,...diff});
   }
   if([1,2,5,7].includes(n))await shot(`${width}-page-${n}`);
  }
  assert(!await evaluate("!!document.querySelector('.comic-reader > details')"));
  const pageHeight=await evaluate("document.querySelector('[data-page-viewport]').getBoundingClientRect().height");
  await evaluate("document.querySelector('[data-reader-dialog=panels]').click()");
  assert(await evaluate("document.querySelector('#comic-panels').matches(':modal')"));
  assert.equal(await evaluate("document.querySelector('[data-page-viewport]').getBoundingClientRect().height"),pageHeight);
  await shot(`${width}-panel-modal`);
  await evaluate("document.querySelector('#comic-panels [data-dialog-close]').click()");await delay(80);
  assert.equal(await evaluate("document.activeElement.dataset.readerDialog"),'panels');
  await evaluate("location.hash='page-5'");await delay(150);
  await evaluate("document.querySelector('[data-reader-dialog=transcript]').click()");
  assert(await evaluate("document.querySelector('#comic-transcript').matches(':modal')"));
  assert.equal(await evaluate("document.querySelector('[data-page-viewport]').getBoundingClientRect().height"),pageHeight);
  assert(await evaluate("document.querySelector('#comic-transcript [data-page-transcript]').textContent.includes('Leila')"));
  await shot(`${width}-transcript-modal`);
  await evaluate("document.querySelector('#comic-transcript .comic-reader-dialog__body').focus()");
  await send('Input.dispatchKeyEvent',{type:'keyDown',key:'PageDown',code:'PageDown',windowsVirtualKeyCode:34});await send('Input.dispatchKeyEvent',{type:'keyUp',key:'PageDown',code:'PageDown',windowsVirtualKeyCode:34});await delay(200);
  assert.equal(await evaluate('location.hash'),'#page-5');
  if(width===390)assert(await evaluate("document.querySelector('#comic-transcript .comic-reader-dialog__body').scrollTop>0"));
  await evaluate("document.querySelector('[data-page-next]').focus()");
  assert(await evaluate("!!document.activeElement.closest('#comic-transcript')"),'Background is inert while the modal is open');
  await send('Input.dispatchKeyEvent',{type:'keyDown',key:'Escape',code:'Escape',windowsVirtualKeyCode:27});await send('Input.dispatchKeyEvent',{type:'keyUp',key:'Escape',code:'Escape',windowsVirtualKeyCode:27});await delay(100);
  assert(!await evaluate("!!document.querySelector('dialog[open]')"));
  assert.equal(await evaluate("document.activeElement.dataset.readerDialog"),'transcript');
  assert(!await evaluate("!!document.querySelector('[data-layout-toggle], [data-layout-overlay], .comic-layout-report')"));
  await evaluate("document.querySelector('[data-art-toggle]').click()");assert.equal(await evaluate("getComputedStyle(document.querySelector('.comic-page:not([hidden]) .comic-lettering')).visibility"),'hidden');
  await evaluate("document.querySelector('[data-art-toggle]').click();location.hash='page-4'");await delay(100);await send('Page.reload',{ignoreCache:true});await delay(400);await wait("document.querySelectorAll('[data-layout-ready=true]').length===7 && document.querySelector('.comic-page:not([hidden])')?.id==='page-4'");
  assert(await evaluate('document.documentElement.scrollWidth<=innerWidth'));
 }
 if(process.env.PUBLIC_ONLY!=='1'){
  await navigate('/__wrap/');await wait("document.querySelectorAll('[data-layout-ready=true]').length===7");const wrapped=await evaluate("[...document.querySelectorAll('#page-1 [data-panel=\\\"1b\\\"] [data-balloon=\\\"0\\\"] text')].slice(1).map(n=>n.textContent)");assert(wrapped.length>1);assert.equal(wrapped.join(' '),"Coffee's still hot. Take a minute before somebody finds you another job.");
  for(const kind of ['script','event','url']){await navigate(`/__unsafe-${kind}/`);await wait("!!document.querySelector('#page-1')?.dataset.layoutError");assert.equal(await evaluate('window.__bad'),undefined);assert(await evaluate("document.querySelector('[data-page-transcript]').textContent.includes('Coffee')"));}
 }
 assert.deepEqual(errors,[]);assert(pixels.every(p=>p.pixels===0));await writeFile(out+'/checks.json',JSON.stringify({noServer:true,singleLibrary:true,seasonRedirect:true,modalChecks:process.env.PUBLIC_ONLY!=='1',noLayoutUi:true,base,reports,pixels,errors},null,2)+'\n');console.log(JSON.stringify(pixels));
}finally{
 browser.kill('SIGTERM');await Promise.race([new Promise(done=>browser.once('exit',done)),delay(3000)]);await writeFile(out+'/browser.txt',log);await rm(profile,{recursive:true,force:true,maxRetries:10,retryDelay:100});
}
