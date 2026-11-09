// Actual WASM at narrow/tablet/landscape sizes. No real credentials or AI calls.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const {fixture,user,turn}=require('./ui-fixtures.cjs');
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
const {execFileSync}=require('node:child_process');
const target=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{cwd:path.resolve(__dirname,'..'),encoding:'utf8'})).target_directory;
const output=path.join(target,'qa-responsive');fs.mkdirSync(output,{recursive:true});
const base=process.env.QA_WEB||'http://127.0.0.1:5173';assert(['localhost','127.0.0.1'].includes(new URL(base).hostname));
const sizes=[[320,568],[360,640],[390,844],[768,1024],[820,1180],[1024,768],[1440,900],[844,390],[390,430]];
const routes=['/','/login','/me','/dict/hello','/translate','/chat/1','/read','/admin/dict','/admin/read'];
const longUser={...user,name:'一个比较长的学习者昵称用来检查布局',email:'long.email.address.for.responsive@example.test'};
const settle=page=>page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
let checks=0;const results=[];let current;
function verify(value,description){assert(value,description);checks++;}
async function setup(context){
 await context.addInitScript(u=>{if(!sessionStorage.getItem('qa-responsive')){localStorage.setItem('colang.token','responsive-fixture');localStorage.setItem('colang.user',JSON.stringify(u));sessionStorage.setItem('qa-responsive','1');}},longUser);
 await context.route('**/api/**',route=>{
  const url=new URL(route.request().url());let data=fixture(url,route.request().method());
  if(url.pathname==='/api/me')data=longUser;
  if(url.pathname==='/api/learn/chats')data=Array.from({length:12},(_,i)=>({id:i+1,title:'英语对话练习 '+(i+1)}));
  if(url.pathname.includes('/turns'))data={items:Array.from({length:20},(_,i)=>({...turn,id:i+1,speaker:i%2?'user':'ai',content_en:'A longer conversation message for reading, scrolling and viewport checks. Message '+(i+1)})),has_prev:false};
  if(url.pathname==='/api/dict/lookup')data={...data,relations:[{relation_type:'synonym',related_word:'hi'}],forms:[{form_type:'plural',form:'hellos'}],etymologies:[{etymology:'An English greeting.'}],dictionaries:[{name_en:'Example dictionary'}]};
  return route.fulfill({json:data});
 });
}
async function loaded(page,url){await page.goto(base+url);await page.locator('main h1,main h2').first().waitFor({state:'attached'});await page.waitForFunction(()=>!document.querySelector('main .loading-status'));await settle(page);}
async function chatBounds(page,height){
 const boxes=await page.evaluate(()=>{const rect=s=>{const r=document.querySelector(s).getBoundingClientRect();return {top:r.top,bottom:r.bottom,height:r.height};};return {feed:rect('.chat-messages'),composer:rect('.chat-composer'),pageHeight:document.documentElement.scrollHeight,layoutHeight:innerHeight};});
 verify(boxes.composer.bottom<=height+1,'chat composer must fit the visible viewport');
 verify(boxes.feed.height>=64,'chat retains a usable message viewport');
 verify(boxes.pageHeight<=boxes.layoutHeight+1,'chat does not require scrolling the outer page');
 return boxes;
}
async function dialogBounds(page,width,height){
 const dialog=page.getByRole('dialog');await dialog.waitFor();await settle(page);
 const bounds=await dialog.boundingBox();verify(bounds.x>=0&&bounds.x+bounds.width<=width+1,'dialog fits width');verify(bounds.y>=0&&bounds.y+bounds.height<=height+1,'dialog fits height');
 await dialog.evaluate(e=>{e.scrollTop=e.scrollHeight;});await settle(page);
 const close=dialog.locator('.sapid-layer-close');const visible=await close.evaluate(e=>{const r=e.getBoundingClientRect();const at=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return {height:r.height,hit:at===e||e.contains(at)};});
 verify(visible.hit&&visible.height>=44,'long forms keep a reachable close button after scrolling');
}
(async()=>{
 const browser=await chromium.launch({headless:true});const errors=[];
 try{
  for(const [width,height] of sizes){
   const context=await browser.newContext({viewport:{width,height}});await setup(context);const page=await context.newPage();current=page;
   page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error'&&m.text().startsWith('Glory frame:'))errors.push(m.text());});
   for(const url of routes){
    await loaded(page,url);
    verify(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),`${url}@${width} has no page-wide overflow`);
    const header=await page.locator('.header').boundingBox();verify(header.height<=70,`${url}@${width} keeps a compact header`);
    if(url==='/')verify(await page.locator('.stat-value').evaluateAll(nodes=>nodes.every(e=>e.getBoundingClientRect().height<=parseFloat(getComputedStyle(e).fontSize)*1.8)),'statistics stay on one line');
    if(url==='/chat/1')await chatBounds(page,height);
    if(url==='/dict/hello'){
     const tabs=page.locator('.word-tabs [role=tablist]');verify(await tabs.getAttribute('aria-orientation')===(width<=950?'horizontal':'vertical'),'dictionary keyboard orientation matches layout');
     if(width<=950){await page.getByRole('tab',{name:'基本释义',exact:true}).focus();await page.keyboard.press('End');await page.waitForFunction(()=>[...document.querySelectorAll('[role=tab]')].some(e=>e.textContent==='词典来源'&&e.getAttribute('aria-selected')==='true'));await page.getByRole('tab',{name:'词典来源',exact:true}).waitFor();verify(await page.getByRole('tab',{name:'词典来源',exact:true}).getAttribute('aria-selected')==='true','horizontal tabs support keyboard selection');}
    }
    if(url==='/translate'&&width<=950){
     const toggle=page.getByRole('button',{name:'翻译设置',exact:true});verify(await toggle.getAttribute('aria-expanded')==='false','mobile translation options start collapsed');await toggle.click();await page.getByRole('button',{name:'商务',exact:true}).click();await toggle.click();await page.waitForFunction(()=>document.querySelector('.translation-options [aria-expanded]')?.getAttribute('aria-expanded')==='false');verify(await toggle.getAttribute('aria-expanded')==='false','translation options collapse without losing state');
    }
    if(url==='/admin/dict'&&width<=600){
     const row=page.locator('.record').first();verify(await row.locator('.cell-label').first().isVisible(),'mobile table rows include field labels');
     const box=await row.boundingBox();verify(box.x>=0&&box.x+box.width<=width+1,'table card fits width');
     for(const name of ['编辑','删除']){const action=await row.getByRole('button',{name,exact:true}).boundingBox();verify(action.x>=0&&action.x+action.width<=width+1&&action.height>=44,'table actions remain visible and touchable');}
    }
    if([320,820].includes(width)||height<500)await page.screenshot({path:path.join(output,`${url.slice(1).replaceAll('/','-')||'home'}-${width}-${height}.png`),fullPage:url!=='/chat/1'});
    results.push({url,width,height,passed:true});
   }
   await loaded(page,'/me#settings');await page.getByRole('button',{name:'添加 AI 配置',exact:true}).click();await page.getByRole('combobox',{name:'AI 平台',exact:true}).fill('OpenAI');await page.getByRole('option',{name:'OpenAI',exact:true}).click();await dialogBounds(page,width,height);
   await page.screenshot({path:path.join(output,`provider-${width}-${height}.png`)});await page.keyboard.press('Escape');
   await loaded(page,'/admin/dict');await page.getByRole('button',{name:'新建记录',exact:true}).click();await dialogBounds(page,width,height);await page.keyboard.press('Escape');
   await context.close();console.log('PASS responsive viewport',width,height);
  }
  const safe=await browser.newContext({viewport:{width:844,height:390}});await setup(safe);const safePage=await safe.newPage();current=safePage;await loaded(safePage,'/chat/1');
  await safePage.evaluate(()=>{const style=document.documentElement.style;style.setProperty('--safe-left','44px');style.setProperty('--safe-right','44px');style.setProperty('--safe-bottom','16px');});await settle(safePage);
  const safeBrand=await safePage.locator('.brand').boundingBox(),safeComposer=await safePage.locator('.chat-composer').boundingBox();verify(safeBrand.x>=44,'landscape navigation avoids the notch');verify(safeComposer.x>=44&&safeComposer.x+safeComposer.width<=800,'landscape composer avoids both side cutouts');verify(safeComposer.y+safeComposer.height<=374,'composer clears the bottom safe area');await safePage.screenshot({path:path.join(output,'landscape-safe-area.png')});await safe.close();
  const context=await browser.newContext({viewport:{width:1280,height:900}});await setup(context);const page=await context.newPage();current=page;
  page.on('pageerror',e=>errors.push(e.message));page.on('console',m=>{if(m.type()==='error'&&m.text().startsWith('Glory frame:'))errors.push(m.text());});
  await loaded(page,'/dict/hello');await page.getByRole('tab',{name:'例句',exact:true}).click();await page.setViewportSize({width:390,height:844});await settle(page);
  verify(await page.getByRole('tab',{name:'例句',exact:true}).getAttribute('aria-selected')==='true','dictionary selection survives a breakpoint change');verify(await page.locator('.word-tabs [role=tablist]').getAttribute('aria-orientation')==='horizontal','orientation updates after resize');
  await page.setViewportSize({width:1280,height:900});await settle(page);verify(await page.locator('.word-tabs [role=tablist]').getAttribute('aria-orientation')==='vertical','desktop tab orientation returns');
  await loaded(page,'/translate');await page.getByRole('button',{name:'商务',exact:true}).click();await page.setViewportSize({width:390,height:844});await settle(page);await page.getByRole('button',{name:'翻译设置',exact:true}).click();verify(await page.getByRole('button',{name:'商务',exact:true}).getAttribute('aria-pressed')==='true','translation choices survive resizing');await context.close();
  // Model an iOS-style visual viewport: layout height stays 844 while the
  // keyboard leaves 430 visible pixels. This is a simulation, not a real IME.
  const keyboard=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true});await setup(keyboard);
  await keyboard.addInitScript(()=>{let height=innerHeight,top=0,scale=1;const viewport=new EventTarget();Object.defineProperties(viewport,{width:{get:()=>innerWidth},height:{get:()=>height},offsetTop:{get:()=>top},offsetLeft:{get:()=>0},pageTop:{get:()=>top},pageLeft:{get:()=>0},scale:{get:()=>scale}});Object.defineProperty(window,'visualViewport',{configurable:true,value:viewport});window.qaViewport=(h,t=0,s=1)=>{height=h;top=t;scale=s;viewport.dispatchEvent(new Event('resize'));};});
  const mobile=await keyboard.newPage();current=mobile;mobile.on('pageerror',e=>errors.push(e.message));
  await loaded(mobile,'/chat/1');await mobile.getByRole('textbox',{name:'输入消息',exact:true}).fill('Keep this unsent draft.');await mobile.evaluate(()=>window.qaViewport(430));await settle(mobile);await chatBounds(mobile,430);
  verify(await mobile.getByRole('textbox',{name:'输入消息',exact:true}).inputValue()==='Keep this unsent draft.','keyboard resize preserves the draft');
  await mobile.route('**/api/learn/chats/1/send/stream',r=>r.fulfill({status:402,json:{error:{brief:'请先配置自己的 AI 服务后再发送消息。'}}}));await mobile.getByRole('button',{name:'发送消息',exact:true}).click();await mobile.getByRole('alert').waitFor();await settle(mobile);
  const failureBox=await mobile.locator('.chat-composer').boundingBox();verify(failureBox.y+failureBox.height<=431,'error feedback does not push the composer behind the keyboard');verify(await mobile.getByRole('textbox',{name:'输入消息',exact:true}).inputValue()==='Keep this unsent draft.','a failed send preserves the draft');
  await mobile.evaluate(()=>window.qaViewport(844));await settle(mobile);await chatBounds(mobile,844);
  await loaded(mobile,'/me#settings');await mobile.getByRole('button',{name:'添加 AI 配置',exact:true}).click();await mobile.getByRole('combobox',{name:'AI 平台',exact:true}).fill('OpenAI');await mobile.getByRole('option',{name:'OpenAI',exact:true}).click();await mobile.evaluate(()=>window.qaViewport(430));await settle(mobile);await dialogBounds(mobile,390,430);
  await mobile.screenshot({path:path.join(output,'keyboard-provider.png')});await keyboard.close();
  verify(errors.length===0,errors.join('; '));fs.writeFileSync(path.join(output,'report.json'),JSON.stringify({sizes,routes:routes.length,audits:results.length,checks,results,errors},null,2));console.log(`${results.length} responsive page/viewport audits and ${checks} layout/interaction checks passed.`);
 }finally{await browser.close();}
})().catch(async e=>{if(current&&!current.isClosed())await current.screenshot({path:path.join(output,'failure.png')}).catch(()=>{});console.error(e);process.exitCode=1;});
