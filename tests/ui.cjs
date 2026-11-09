// Cross-page visual/interaction contract. Browser uses the actual built WASM;
// deterministic API fixtures expose every state without contacting AI providers.
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const artifacts = JSON.parse(execFileSync('cargo', ['metadata','--no-deps','--format-version','1'], {cwd:path.resolve(__dirname,'..'),encoding:'utf8'})).target_directory;
const output = path.join(artifacts,'qa-ui'); fs.mkdirSync(output,{recursive:true});
const base = process.env.QA_WEB || 'http://127.0.0.1:5173';
assert(['localhost','127.0.0.1'].includes(new URL(base).hostname));
const user = {id:123,name:'界面复检账户',email:'ui@example.test',is_admin:true};
const word = {id:1,word:'hello',is_active:true};
const sentence = {id:1,subject_id:1,sentence_order:1,content_en:'Hello world.',content_zh:'你好，世界。',difficulty:1};
const turn = {id:1,speaker:'ai',content_en:'Hello! Let us practice English.',content_zh:'你好，让我们开始练习。',status:'completed',issues:[]};
const review = {id:1,word:'hello',word_zh:'你好',example:'Say hello to a friend.',example_zh:'向朋友问好。',review_type:'recognition',source:'vocabulary',source_id:1};
const domain = {id:1,code:'daily',name_en:'Daily life',name_zh:'日常生活'};
const plan = {id:1,code:'free',name_zh:'基础版',description_zh:'每日学习，持续积累。',price_display:'免费',trial_days:0,features:['基础词典','词汇复习']};
function fixture(url) {
  const p=url.pathname;
  if(p==='/api/me')return user;
  if(p==='/api/dict/lookup')return {word,definitions:[{definition:'你好',part_of_speech:'interj.'}],sentences:[{sentence:'Hello world.'}]};
  if(p==='/api/dict/words')return {words:[word],total:1,per_page:30};
  if(p==='/api/dict/words/1')return word;
  if(p.includes('/definitions'))return [{id:1,language:'zh',definition:'你好',is_summary:true}];
  if(p.includes('/pronunciations'))return [{id:1,ipa:'həˈləʊ',dialect:'British'}];
  if(p==='/api/learn/chats')return [{id:1,title:'英语对话练习'}];
  if(p.includes('/turns'))return {items:[turn],total:1,has_prev:false,has_next:false};
  if(p==='/api/asset/contexts')return [{id:1,name_zh:'日常交流',icon_emoji:'☀'}];
  if(p==='/api/learn/review/items')return {items:[review],total:1};
  if(p==='/api/learn/review/stats')return {total:5,recognition:1,meaning:1,spelling:1,phrase:1,pronunciation:1};
  if(p.includes('/read/recommended'))return {items:[sentence],user_level:1};
  if(p==='/api/asset/read/subjects')return {items:[{id:1,title_zh:'日常跟读'}]};
  if(p==='/api/admin/read-subjects')return {subjects:[{id:1,code:'daily',title_zh:'日常跟读',title_en:'Daily reading'}],total:1};
  if(p.includes('/read-sentences'))return {sentences:[sentence],total:1};
  if(p.endsWith('/scripts'))return [{id:1,title_zh:'问好',title_en:'Greeting'}];
  if(p.endsWith('/stages'))return p.includes('/admin/')?{stages:[{...domain,is_active:true}],total:1}:[{...domain,is_active:true}];
  if(p.includes('/stages/1'))return domain;
  if(p.includes('/domains'))return p.endsWith('/1')?domain:{domains:[domain],total:1};
  if(p.includes('/categories'))return p.endsWith('/1')?{...domain,domain_id:1}:{categories:[{...domain,domain_id:1}],total:1};
  if(p==='/api/learn/favorites')return [{...word,entity:'word',record_id:1}];
  if(p==='/api/subscription/plans')return [plan];
  if(p==='/api/subscription/current')return {has_subscription:false};
  if(p==='/api/wallet')return {account:{balance:0}};
  if(p.includes('/transactions'))return {items:[],total:0};
  if(p==='/api/learn/daily')return {tasks:[{id:1,task_title:'完成一次英语练习',target_value:1,status:'pending'}]};
  if(p==='/api/learn/summary')return {learning_days:7,total_vocabulary_count:32,pending_review_count:5};
  if(p==='/api/me/account-status')return {has_password:true,linked_identities:[]};
  if(p==='/api/ai-providers')return [{id:1,provider_type:'zhipu',is_active:true,settings:{chat_model:'example-model'}}];
  if(p==='/api/admin/users')return {users:[user],total:1};
  if(p.startsWith('/api/admin/users/'))return {user};
  if(p.includes('/stats'))return {total:1};
  return [];
}
const settle = p => p.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
async function color(locator) {return locator.evaluate(e=>{const s=getComputedStyle(e);return {background:s.backgroundColor,color:s.color,border:s.borderTopColor,outline:s.outlineColor,font:s.fontSize,busy:e.getAttribute('aria-busy'),disabled:e.disabled};});}
let browser,page,checks=0;
const results=[];
async function scenario(name,run){await run();checks++;results.push({name,status:'passed'});console.log('PASS',name);}
(async()=>{
  browser=await chromium.launch({headless:true,args:['--use-fake-device-for-media-stream','--use-fake-ui-for-media-stream']});
  const context=await browser.newContext({viewport:{width:1280,height:900},permissions:['microphone']});
  page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));page.on('dialog',d=>d.accept());
  await page.route('**/api/**',route=>route.fulfill({json:fixture(new URL(route.request().url()))}));
  await page.goto(base);await page.evaluate(()=>localStorage.setItem('colang.token','ui-fixture'));
  const routes=['/','/dict','/dict/hello','/translate','/chat','/chat/1','/review',...['meaning','spelling','recognition','phrase','pronunciation'].map(s=>'/review/session/'+s),'/stages','/stages/1','/read','/login','/auth','/me','/favorites','/subscription','/admin','/admin/users','/admin/users/123','/admin/finance','/admin/finance/orders','/admin/finance/subscriptions','/admin/dict','/admin/dict/1','/admin/taxonomy','/admin/taxonomy/domains/1','/admin/taxonomy/categories/1','/admin/chat-scenarios','/admin/read','/feedback','/terms','/privacy','/missing'];
  await scenario('37 routes: desktop/mobile theme, typography, focus and overflow',async()=>{
    for(const width of [1280,390]){
      await page.setViewportSize({width,height:900});
      for(const route of routes){
        await page.goto(base+route);await page.locator('main h1,main h2').first().waitFor();await page.waitForFunction(()=>!document.querySelector('main .loading-status'));await settle(page);
        const audit=await page.evaluate(()=>{
          const blue=c=>{const v=c.match(/[\d.]+/g)?.map(Number);return v&&v[2]>v[0]*1.3&&v[2]>v[1]*1.15&&v[2]>100;};
          const controls=[...document.querySelectorAll('main button,main input,main textarea')];
          return {overflow:document.documentElement.scrollWidth>innerWidth+1,wrongTheme:controls.filter(e=>!e.closest('.sapid-theme-colang-light')).length,blue:controls.filter(e=>{const s=getComputedStyle(e);return [s.color,s.backgroundColor,s.borderTopColor,s.accentColor].some(blue);}).length,buttonFonts:[...document.querySelectorAll('main button')].map(e=>getComputedStyle(e).fontSize),inputFonts:[...document.querySelectorAll('main input:not([type=checkbox]),main textarea')].map(e=>getComputedStyle(e).fontSize),heading:document.querySelector('main h1')?getComputedStyle(document.querySelector('main h1')).fontSize:null,auth:!!document.querySelector('.auth-card'),current:[...document.querySelectorAll('.header a[aria-current],.admin-nav a[aria-current]')].map(e=>e.textContent),text:document.querySelector('main').textContent};
        });
        assert(!audit.overflow,`${route}@${width} overflows`);assert.equal(audit.wrongTheme,0,`${route} has an unthemed control`);assert.equal(audit.blue,0,`${route} has blue controls`);
        assert(audit.buttonFonts.every(s=>s==='16px'),`${route}: button fonts ${audit.buttonFonts}`);assert(audit.inputFonts.every(s=>s==='16px'),`${route}: input fonts ${audit.inputFonts}`);
        if(audit.auth)assert.equal(audit.heading,width===1280?'26px':'24px',route);
        else if(route!=='/'&&audit.heading)assert.equal(audit.heading,width===1280?'32px':'26px',route);
        assert(!audit.text.includes('页面加载失败'),route);
        if(route.startsWith('/dict'))assert(audit.current.includes('词典'));
        if(route.startsWith('/admin'))assert(audit.current.includes('管理'));
        if(['/login','/translate','/chat/1','/read','/me','/admin/read'].includes(route))await page.screenshot({path:path.join(output,route.slice(1).replaceAll('/','-')+'-'+width+'.png'),fullPage:true});
      }
    }
  });
  await scenario('login validation, keyboard focus, loading lock and credential error recovery',async()=>{
    await page.evaluate(()=>localStorage.removeItem('colang.token'));await page.goto(base+'/login');const submit=page.getByRole('button',{name:'登录',exact:true}).last();
    await submit.click();await page.getByRole('alert').waitFor();assert.equal(await page.getByRole('textbox',{name:'邮箱',exact:true}).getAttribute('aria-invalid'),'true');
    const email=page.getByRole('textbox',{name:'邮箱',exact:true});await email.focus();await page.waitForFunction(()=>getComputedStyle(document.querySelector('input[aria-invalid=true]')).borderTopColor==='rgb(185, 28, 28)');assert.equal((await color(email)).outline,'rgb(194, 65, 12)');assert.equal((await color(email)).border,'rgb(185, 28, 28)');
    await email.fill('ui@example.test');await page.getByRole('textbox',{name:'密码',exact:true}).fill('password');
    let requests=0;await page.route('**/api/login',async route=>{requests++;await new Promise(r=>setTimeout(r,500));await route.fulfill({status:401,json:{error:{brief:'invalid password'}}});});
    await submit.click();await page.waitForFunction(()=>document.querySelector('.auth-card .sapid-button-primary')?.getAttribute('aria-busy')==='true');assert(await email.isDisabled());assert(await page.getByRole('button',{name:'注册',exact:true}).isDisabled());await page.getByText('邮箱或密码不正确，请检查后重试',{exact:true}).waitFor();assert.equal(requests,1);assert(!(await email.isDisabled()));await page.unroute('**/api/login');
    await page.getByRole('button',{name:'注册',exact:true}).click();await settle(page);assert.equal(await page.getByRole('alert').count(),0);
  });
  await scenario('favorites category resets pagination and clears stale records on load failure',async()=>{
    await page.evaluate(()=>localStorage.setItem('colang.token','ui-fixture'));let offsets=[];
    await page.route('**/api/learn/favorites?*',async route=>{const u=new URL(route.request().url());offsets.push([u.searchParams.get('entity'),u.searchParams.get('offset')]);if(u.searchParams.get('entity')==='sentence'){await new Promise(r=>setTimeout(r,250));return route.fulfill({status:503,json:{error:{brief:'temporary failure'}}});}return route.fulfill({json:u.searchParams.get('offset')==='0'?Array.from({length:30},(_,i)=>({id:i,record_id:i,entity:'word',word:'favorite-'+i})):[]});});
    await page.goto(base+'/favorites');await page.getByRole('heading',{name:'favorite-0',exact:true}).waitFor();await page.getByRole('button',{name:'下一页',exact:true}).click();await page.getByText('还没有收藏，去词典或跟读中添加吧。',{exact:true}).waitFor();await page.getByRole('button',{name:'句子',exact:true}).click();await page.getByRole('alert').waitFor();assert.deepEqual(offsets.at(-1),['sentence','0']);assert.equal(await page.locator('.empty-state').count(),0);assert.equal(await page.getByRole('heading',{name:'favorite-0',exact:true}).count(),0);await page.unroute('**/api/learn/favorites?*');
  });
  await scenario('admin validation and processing locks use shared error/danger states',async()=>{
    await page.goto(base+'/admin/taxonomy');const section=page.locator('.admin-section').first();await section.getByRole('button',{name:'保存记录',exact:true}).click();await section.getByRole('alert').waitFor();assert.equal(await section.getByRole('textbox',{name:'代码 *',exact:true}).getAttribute('aria-invalid'),'true');
    await section.getByRole('textbox',{name:'代码 *',exact:true}).fill('ui-new');await section.getByRole('textbox',{name:'英文名称 *',exact:true}).fill('UI new');await section.getByRole('textbox',{name:'中文名称 *',exact:true}).fill('界面测试');
    await page.route('**/api/asset/domains',async route=>{if(route.request().method()==='POST'){await new Promise(r=>setTimeout(r,400));return route.fulfill({status:500,json:{error:{brief:'save failed'}}});}return route.fulfill({json:fixture(new URL(route.request().url()))});});
    await section.getByRole('button',{name:'保存记录',exact:true}).click();await page.waitForFunction(()=>document.querySelector('.editor input')?.disabled);assert(await section.getByRole('button',{name:'新建 / 取消编辑',exact:true}).isDisabled());await section.getByText('save failed',{exact:true}).waitFor();assert.equal(await section.getByRole('textbox',{name:'代码 *',exact:true}).inputValue(),'ui-new');await page.unroute('**/api/asset/domains');
  });
  await scenario('recording and evaluation clear prior scores and prevent stale practice saves',async()=>{
    let evaluations=0;await page.route('**/api/learn/pronunciation/evaluate',async route=>{evaluations++;if(evaluations===2){await new Promise(r=>setTimeout(r,400));return route.fulfill({status:503,json:{error:{brief:'evaluation unavailable'}}});}return route.fulfill({json:{overall_score:88,pronunciation_score:88,fluency_score:88,intonation_score:88,feedback:['第一次评分']}});});
    await page.goto(base+'/read');await page.getByRole('button',{name:'开始录音',exact:true}).click();await page.getByRole('button',{name:'停止录音并提交',exact:true}).waitFor();await page.waitForTimeout(300);await page.getByRole('button',{name:'停止录音并提交',exact:true}).click();await page.getByText('第一次评分',{exact:false}).waitFor();const save=page.getByRole('button',{name:'保存本次成绩',exact:true});assert(!(await save.isDisabled()));
    await page.getByRole('button',{name:'开始录音',exact:true}).click();await page.getByRole('button',{name:'停止录音并提交',exact:true}).waitFor();assert(await save.isDisabled());assert.equal(await page.getByText('第一次评分',{exact:false}).count(),0);await page.waitForTimeout(300);await page.getByRole('button',{name:'停止录音并提交',exact:true}).click();await page.getByText('evaluation unavailable',{exact:true}).waitFor();assert(await save.isDisabled());assert(!(await page.getByRole('button',{name:'开始录音',exact:true}).isDisabled()));await page.unroute('**/api/learn/pronunciation/evaluate');
  });
  await scenario('keyboard/reduced motion and 200-percent-equivalent narrow layout',async()=>{
    await page.goto(base+'/login');await page.setViewportSize({width:320,height:700});await page.emulateMedia({reducedMotion:'reduce'});await page.getByRole('textbox',{name:'邮箱',exact:true}).focus();await page.keyboard.press('Tab');const focus=await page.locator(':focus').evaluate(e=>({name:e.getAttribute('aria-label'),outline:getComputedStyle(e).outlineColor}));assert.equal(focus.name,'密码');assert.equal(focus.outline,'rgb(194, 65, 12)');assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));
  });
  await scenario('changing reading subjects releases microphone ownership',async()=>{
    await page.route('**/api/asset/read/recommended?*',route=>route.fulfill({json:{items:[{...sentence,id:new URL(route.request().url()).searchParams.has('subject_id')?2:1}],user_level:1}}));
    await page.goto(base+'/read');await page.evaluate(()=>{const original=navigator.mediaDevices.getUserMedia.bind(navigator.mediaDevices);window.qaTracks=[];navigator.mediaDevices.getUserMedia=async options=>{const stream=await original(options);window.qaTracks.push(...stream.getTracks());return stream;};});
    await page.getByRole('button',{name:'开始录音',exact:true}).click();await page.getByRole('button',{name:'停止录音并提交',exact:true}).waitFor();await page.getByRole('button',{name:'日常跟读',exact:true}).click();await page.getByRole('button',{name:'开始录音',exact:true}).waitFor();assert(await page.evaluate(()=>window.qaTracks.length>0&&window.qaTracks.every(track=>track.readyState==='ended')));
    await page.getByRole('button',{name:'开始录音',exact:true}).click();await page.getByRole('button',{name:'停止录音并提交',exact:true}).waitFor();await page.goto(base+'/review');await page.unroute('**/api/asset/read/recommended?*');
  });
  await scenario('session expiry removes protected content immediately',async()=>{
    await page.goto(base+'/chat/1');await page.getByRole('textbox',{name:'输入消息',exact:true}).fill('Hello');await page.route('**/api/learn/chats/1/send',route=>route.fulfill({status:401,json:{error:{brief:'expired token'}}}));await page.getByRole('button',{name:'发送消息',exact:true}).click();await page.getByRole('heading',{name:'登录后继续学习',exact:true}).waitFor();assert.equal(await page.getByRole('textbox',{name:'输入消息',exact:true}).count(),0);assert.equal(await page.evaluate(()=>localStorage.getItem('colang.token')),null);await page.unroute('**/api/learn/chats/1/send');
  });
  await scenario('primary, selected, disabled and danger components share state colors',async()=>{
    await page.setViewportSize({width:1280,height:900});await page.emulateMedia({reducedMotion:'reduce'});await page.goto(base+'/login');const primary=page.getByRole('button',{name:'登录',exact:true}).last();await page.mouse.move(0,0);assert.equal((await color(primary)).background,'rgb(194, 65, 12)');await primary.hover();assert.equal((await color(primary)).background,'rgb(154, 52, 18)');await page.mouse.down();assert.equal((await color(primary)).background,'rgb(124, 45, 18)');await page.mouse.move(0,0);await page.mouse.up();await page.getByRole('button',{name:'注册',exact:true}).click();await settle(page);assert.equal((await color(page.getByRole('button',{name:'注册',exact:true}))).background,'rgb(255, 237, 213)');assert.equal(await page.getByRole('checkbox').evaluate(e=>getComputedStyle(e).accentColor),'rgb(194, 65, 12)');
    await page.evaluate(()=>localStorage.setItem('colang.token','ui-fixture'));await page.goto(base+'/admin/dict');const danger=page.getByRole('button',{name:'批量删除',exact:true});await danger.waitFor();assert.equal((await color(danger)).background,'rgb(245, 245, 244)');await page.getByRole('button',{name:'选择本页全部单词',exact:true}).click();await settle(page);assert.equal((await color(danger)).background,'rgb(185, 28, 28)');
  });
  await scenario('AI connection errors are red and successful retries are green',async()=>{
    await page.goto(base+'/me');await page.route('**/api/ai-providers/1/test',route=>route.fulfill({json:{success:false,error:'测试连接失败'}}));await page.getByRole('button',{name:'测试连接',exact:true}).click();await page.getByRole('alert').getByText('测试连接失败',{exact:true}).waitFor();assert.equal(await page.locator('.success').count(),0);await page.unroute('**/api/ai-providers/1/test');await page.route('**/api/ai-providers/1/test',route=>route.fulfill({json:{success:true}}));await page.getByRole('button',{name:'测试连接',exact:true}).click();await page.locator('.success').getByText('连接成功',{exact:true}).waitFor();assert.equal(await page.getByRole('alert').count(),0);await page.unroute('**/api/ai-providers/1/test');
  });
  await scenario('admin searches reset pagination before requesting filtered data',async()=>{
    const queries=[];await page.route('**/api/admin/users?*',route=>{const u=new URL(route.request().url());queries.push([u.searchParams.get('page'),u.searchParams.get('search')]);return route.fulfill({json:{users:[user],total:61,per_page:30}});});await page.goto(base+'/admin/users');await page.getByRole('button',{name:'下一页',exact:true}).click();await page.getByText('第 2 页',{exact:true}).waitFor();await page.getByRole('textbox',{name:'搜索用户',exact:true}).fill('filtered');await page.getByText('第 1 页',{exact:true}).waitFor();await page.waitForFunction(()=>!document.querySelector('main .loading-status'));assert.deepEqual(queries.at(-1),['1','filtered']);await page.unroute('**/api/admin/users?*');
  });
  assert.deepEqual(errors,[]);fs.writeFileSync(path.join(output,'report.json'),JSON.stringify({pages:routes.length,viewports:[1280,390],audits:routes.length*2,results},null,2));for(const name of ['failure.png','failure.txt']){const file=path.join(output,name);if(fs.existsSync(file))fs.unlinkSync(file);}console.log(`${routes.length*2} page/viewport audits and ${checks-1} interaction regressions passed.`);
})().catch(async error=>{if(page){await page.screenshot({path:path.join(output,'failure.png'),fullPage:true}).catch(()=>{});fs.writeFileSync(path.join(output,'failure.txt'),await page.locator('body').innerText().catch(()=>''));}console.error(error);process.exitCode=1;}).finally(async()=>{await browser?.close();});
