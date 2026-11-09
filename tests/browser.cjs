// UI regression uses the built WASM app and real local API/DB. Only external AI
// responses are intercepted; request payloads and microphone WAVs are asserted.
const {chromium} = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const artifacts = JSON.parse(require('node:child_process').execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{cwd:path.resolve(__dirname,'..'),encoding:'utf8'})).target_directory;
const fixture = JSON.parse(fs.readFileSync(path.join(artifacts,'qa-fixture.json'),'utf8'));
const base = process.env.QA_WEB || 'http://127.0.0.1:5173';
assert(['127.0.0.1','localhost'].includes(new URL(base).hostname));
const screenshotDir=path.join(artifacts,'qa-screenshots');fs.mkdirSync(screenshotDir,{recursive:true});
let browser, count=0;
async function test(name, run){await run();count++;console.log('PASS',name);}
async function pageFor(auth){
  const context=await browser.newContext({viewport:{width:1280,height:900},permissions:['microphone']});
  const page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));page.on('dialog',d=>d.accept());
  await page.goto(base);
  if(auth)await page.evaluate(auth=>{localStorage.setItem('colang.token',auth.access_token);localStorage.setItem('colang.user',JSON.stringify(auth.user));},auth);
  page.qaClose=async()=>{assert.deepEqual(errors,[]);await context.close();};return page;
}
async function text(page,value){await page.getByText(value,{exact:false}).first().waitFor({timeout:15000});}
async function api(method,endpoint,body,auth=fixture.learner){
  const response=await fetch(base+endpoint,{method,headers:{'Content-Type':'application/json',Authorization:`Bearer ${auth.access_token}`},body:body?JSON.stringify(body):undefined});
  const raw=await response.text();assert(response.ok,`${endpoint} ${response.status} ${raw}`);return raw?JSON.parse(raw):null;
}
(async()=>{
  browser=await chromium.launch({headless:true,args:['--use-fake-ui-for-media-stream','--use-fake-device-for-media-stream']});
  await test('landing, legal pages, 404 and responsive layout',async()=>{
    const p=await pageFor();await text(p,'让英语，成为你的日常。');assert(await p.locator('.brand img').evaluate(e=>e.complete&&e.naturalWidth>0));
    await p.screenshot({path:path.join(screenshotDir,'landing-desktop.png'),fullPage:true});
    await p.setViewportSize({width:390,height:844});assert(await p.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));await p.screenshot({path:path.join(screenshotDir,'landing-mobile.png'),fullPage:true});
    for(const [route,title] of [['/terms','服务条款'],['/privacy','隐私政策'],['/feedback','意见反馈'],['/not-a-page','页面不存在']]){await p.goto(base+route);await p.getByRole('heading',{name:title,exact:true}).waitFor();}
    await p.qaClose();
  });
  await test('protected routes and damaged saved user data',async()=>{
    const p=await pageFor();await p.evaluate(()=>localStorage.setItem('colang.user','{broken'));
    for(const route of ['/me','/chat/1','/favorites','/review/session/spelling','/read','/admin']){await p.goto(base+route);await text(p,'登录后继续学习');}
    await p.qaClose();
  });
  await test('real email login, profile and logout',async()=>{
    const p=await pageFor();await p.goto(base+'/login');await p.getByRole('textbox',{name:'邮箱',exact:true}).fill(fixture.email);await p.getByRole('textbox',{name:'密码',exact:true}).fill(fixture.password);await p.getByRole('button',{name:'登录',exact:true}).last().click();
    const updatedName='QA Updated Learner '+fixture.learner.user.id;
    await p.waitForURL('**/me');await text(p,'我的学习');await p.getByRole('textbox',{name:'昵称',exact:true}).fill(updatedName);await p.getByRole('button',{name:'保存资料',exact:true}).click();await text(p,'资料已保存');
    await p.reload();await p.getByRole('textbox',{name:'昵称',exact:true}).waitFor();assert.equal(await p.getByRole('textbox',{name:'昵称',exact:true}).inputValue(),updatedName);
    await p.getByRole('button',{name:'退出登录',exact:true}).click();await p.waitForURL(base+'/');assert.equal(await p.evaluate(()=>localStorage.getItem('colang.token')),null);await p.qaClose();
  });
  await test('registration validates confirmation and creates a real account',async()=>{
    const p=await pageFor();const stamp=Date.now();await p.goto(base+'/login');await p.getByRole('button',{name:'注册',exact:true}).click();
    await p.getByRole('textbox',{name:'邮箱',exact:true}).fill(`browser-${stamp}@example.test`);await p.getByRole('textbox',{name:'密码',exact:true}).fill(fixture.password);await p.getByRole('textbox',{name:'昵称',exact:true}).fill('Browser Learner '+stamp);await p.getByRole('textbox',{name:'再次输入密码',exact:true}).fill('wrong');await p.getByRole('checkbox').check();await p.getByRole('button',{name:'创建账户',exact:true}).click();await text(p,'一致的密码');
    await p.getByRole('textbox',{name:'再次输入密码',exact:true}).fill(fixture.password);await p.getByRole('button',{name:'创建账户',exact:true}).click();await p.waitForURL('**/me');await text(p,'我的学习');await p.qaClose();
  });
  await test('real avatar upload, authenticated image and deletion',async()=>{
    const p=await pageFor(fixture.learner);await p.goto(base+'/me');const chooser=p.waitForEvent('filechooser');await p.getByRole('button',{name:'上传头像（居中裁剪）',exact:true}).click();await (await chooser).setFiles(path.resolve(__dirname,'../front/public/colang-logo.png'));
    await p.waitForFunction(()=>document.querySelector('.profile-avatar')?.src.startsWith('data:image/'));await p.reload();await p.waitForFunction(()=>document.querySelector('.profile-avatar')?.src.startsWith('data:image/'));await p.getByRole('button',{name:'删除头像',exact:true}).click();await text(p,'头像已删除');await p.waitForFunction(()=>document.querySelector('.profile-avatar')?.src.endsWith('/colang-logo.svg'));await p.qaClose();
  });
  await test('dictionary lookup, deep link, favorite and removal',async()=>{
    const p=await pageFor(fixture.learner);await p.goto(base+'/dict');await p.getByRole('textbox',{name:'单词或短语',exact:true}).fill(fixture.word.word);await p.getByRole('button',{name:'查询',exact:true}).click();await text(p,'回归测试单词');
    await p.getByRole('button',{name:'收藏 / 取消收藏',exact:true}).click();await p.getByText('已收藏',{exact:true}).waitFor();await p.reload();await text(p,'回归测试单词');
    await p.goto(base+'/favorites');await p.getByRole('heading',{name:fixture.word.word,exact:true}).waitFor();await p.getByRole('button',{name:'移出收藏',exact:true}).click();await text(p,'还没有收藏');await p.qaClose();
  });
  await test('latest search wins when earlier response arrives late',async()=>{
    const p=await pageFor();
    await p.route('**/api/dict/lookup?*',async route=>{const q=new URL(route.request().url()).searchParams.get('word');if(q==='slow')await new Promise(r=>setTimeout(r,700));await route.fulfill({json:{word:{id:q==='slow'?9001:9002,word:q},definitions:[{definition:q+' definition'}]}});});
    await p.goto(base+'/dict');const field=p.getByRole('textbox',{name:'单词或短语',exact:true});await field.fill('slow');await p.getByRole('button',{name:'查询',exact:true}).click();
    // Query is disabled while loading, preventing accidental overlapping searches.
    await p.waitForFunction(()=>[...document.querySelectorAll('button')].some(b=>b.textContent==='查询'&&b.disabled));await text(p,'slow definition');await field.fill('fast');await p.getByRole('button',{name:'查询',exact:true}).click();await text(p,'fast definition');assert.equal(await p.getByRole('heading',{name:'slow',exact:true}).count(),0);await p.qaClose();
  });
  await test('translation options, safe text rendering and API failure',async()=>{
    const p=await pageFor(fixture.learner);let payload;
    await p.route('**/api/dict/translate',async route=>{payload=route.request().postDataJSON();await route.fulfill({json:{translated:'<img src=x onerror="window.xss=1"> 测试译文',detected_source:'en'}});});
    await p.goto(base+'/translate');await p.getByRole('textbox',{name:'需要翻译的内容',exact:true}).fill('Hello world');await p.getByRole('button',{name:'商务',exact:true}).click();await p.getByRole('button',{name:'翻译',exact:true}).click();await text(p,'测试译文');assert.equal(payload.options.tone,'business');assert.equal(payload.target,'zh');assert.equal(await p.evaluate(()=>window.xss),undefined);
    await p.unroute('**/api/dict/translate');await p.route('**/api/dict/translate',route=>route.fulfill({status:503,json:{error:{brief:'AI service not available'}}}));await p.getByRole('button',{name:'翻译',exact:true}).click();await text(p,'AI service not available');await p.qaClose();
  });
  await test('review persists result and advances once',async()=>{
    await api('POST','/api/learn/vocabulary',{word:'qa-review-'+Date.now()});
    const before=await api('GET','/api/learn/review/items?type=recognition&limit=100');const first=before.items[0];
    const p=await pageFor(fixture.learner);await p.goto(base+'/review/session/recognition');await p.getByRole('button',{name:'显示释义',exact:true}).waitFor();await p.getByRole('button',{name:'显示释义',exact:true}).click();await p.getByRole('button',{name:'已掌握',exact:true}).click();
    if(before.items.length===1){await text(p,'本轮练习已完成');await text(p,'答对 1 项');}
    else{await p.waitForFunction(word=>document.querySelector('.practice-card h2')?.textContent!==word,first.word);}
    const after=await api('GET','/api/learn/review/items?type=recognition&limit=100');assert.equal(after.items.length,before.items.length-1);assert(!after.items.some(item=>item.source_id===first.source_id));await p.qaClose();
  });
  await test('real microphone -> mono 16k WAV -> pronunciation -> practice save',async()=>{
    const p=await pageFor(fixture.learner);let wav;
    await p.route('**/api/asset/read/recommended?*',route=>route.fulfill({json:{items:[fixture.sentence],user_level:1}}));
    await p.route('**/api/learn/pronunciation/evaluate',route=>{const body=route.request().postDataJSON();assert.equal(body.reference_text,'Hello world.');wav=Buffer.from(body.audio_base64,'base64');return route.fulfill({json:{transcribed_text:'Hello world.',overall_score:85,pronunciation_score:85,fluency_score:90,intonation_score:80,feedback:['继续练习']}});});
    await p.goto(base+'/read');await p.getByRole('button',{name:'开始录音',exact:true}).click();await p.getByRole('button',{name:'停止录音并提交',exact:true}).waitFor();await p.waitForTimeout(1200);await p.getByRole('button',{name:'停止录音并提交',exact:true}).click();await text(p,'继续练习');assert.equal(wav.toString('ascii',0,4),'RIFF');assert.equal(wav.readUInt32LE(24),16000);assert.equal(wav.readUInt16LE(22),1);assert(wav.length>44);
    await p.getByRole('button',{name:'保存本次成绩',exact:true}).click();await text(p,'成绩已保存');assert(await p.getByRole('button',{name:'已保存',exact:true}).isDisabled());await p.qaClose();
  });
  await test('chat create, asynchronous AI reply polling and title update',async()=>{
    const p=await pageFor(fixture.learner);await p.goto(base+'/chat');await p.getByRole('textbox',{name:'对话标题',exact:true}).fill('Browser QA Chat');await p.getByRole('button',{name:'自由对话',exact:true}).click();await p.waitForURL(/\/chat\/\d+$/);const id=Number(p.url().split('/').at(-1));
    let turns=[],polls=0;await p.route(`**/api/learn/chats/${id}/turns?*`,route=>route.fulfill({json:{items:turns,total:turns.length,has_prev:false}}));
    await p.route(`**/api/learn/chats/${id}/send`,route=>{assert.equal(route.request().postDataJSON().message,'Hello there');turns=[{id:900001,speaker:'user',content_en:'Hello there',status:'completed'},{id:900002,speaker:'ai',content_en:'',status:'processing'}];return route.fulfill({json:{user_turn:turns[0],ai_turn:turns[1]}});});
    await p.route('**/api/learn/chats/turns/900002',route=>{polls++;turns[1]={...turns[1],content_en:'Hello! How are you?',content_zh:'你好！',status:'completed',issues:[]};return route.fulfill({json:turns[1]});});
    await p.getByRole('textbox',{name:'输入消息',exact:true}).fill('Hello there');await p.getByRole('button',{name:'发送消息',exact:true}).click();await text(p,'Hello! How are you?');assert.equal(polls,1);
    await p.getByRole('textbox',{name:'对话标题',exact:true}).fill('Browser Renamed');await p.getByRole('button',{name:'修改标题',exact:true}).click();await p.getByRole('link',{name:'Browser Renamed',exact:true}).waitFor();await p.qaClose();
  });
  await test('admin create, edit, delete domain and word deep link',async()=>{
    const p=await pageFor(fixture.admin);await p.goto(base+'/admin/taxonomy');const section=p.locator('.admin-section').first();const code='browser'+Date.now();
    await section.getByRole('textbox',{name:'代码 *',exact:true}).fill(code);await section.getByRole('textbox',{name:'英文名称 *',exact:true}).fill('Browser Domain '+code);await section.getByRole('textbox',{name:'中文名称 *',exact:true}).fill('浏览器测试领域 '+code);await section.getByRole('button',{name:'保存记录',exact:true}).click();await section.getByText('记录已保存',{exact:true}).waitFor();
    let row=section.locator('.record').filter({hasText:code});await row.getByRole('button',{name:'编辑',exact:true}).click();await section.getByRole('textbox',{name:'中文名称 *',exact:true}).fill('已编辑的领域 '+code);await section.getByRole('button',{name:'保存记录',exact:true}).click();await text(row,'已编辑的领域');await row.getByRole('button',{name:'删除',exact:true}).click();await section.getByText('记录已删除',{exact:true}).waitFor();assert.equal(await section.locator('.record').filter({hasText:code}).count(),0);
    await p.goto(base+'/admin/dict/'+fixture.word.id);await p.getByRole('textbox',{name:'单词 *',exact:true}).waitFor();await p.waitForFunction(word=>document.querySelector('.editor input')?.value===word,fixture.word.word);await text(p,'回归测试单词');await p.qaClose();
  });
  await test('all admin deep routes and public scene/subscription routes',async()=>{
    const p=await pageFor(fixture.admin);
    for(const route of ['/admin','/admin/users',`/admin/users/${fixture.learner.user.id}`,'/admin/finance','/admin/finance/orders','/admin/finance/subscriptions','/admin/chat-scenarios','/admin/read',`/admin/taxonomy/domains/1`,`/stages/${fixture.stage.id}`,'/stages','/subscription']){await p.goto(base+route);await p.locator('main').waitFor();await p.waitForTimeout(300);assert(!(await p.locator('body').innerText()).includes('页面加载失败'),route);}
    await p.setViewportSize({width:390,height:844});assert(await p.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1));await p.screenshot({path:path.join(screenshotDir,'subscription-mobile.png'),fullPage:true});await p.qaClose();
  });
  await test('dictionary bulk deactivate and delete preserve server state',async()=>{
    const prefix='batchqa'+Date.now();const words=[];
    for(const suffix of ['a','b'])words.push(await api('POST','/api/dict/words',{word:prefix+suffix,is_active:true},fixture.admin));
    const p=await pageFor(fixture.admin);await p.goto(base+'/admin/dict');const section=p.locator('.admin-section').first();await section.getByRole('textbox',{name:'搜索内容',exact:true}).fill(prefix);await section.locator('.record').filter({hasText:prefix+'a'}).waitFor();
    await section.getByRole('button',{name:'选择本页全部单词',exact:true}).click();await section.getByRole('button',{name:'批量停用',exact:true}).click();await text(section,'已处理 2 项');
    for(const word of words)assert.equal((await api('GET','/api/dict/words/'+word.id,null,fixture.admin)).is_active,false);
    await section.getByRole('button',{name:'选择本页全部单词',exact:true}).click();await section.getByRole('button',{name:'批量删除',exact:true}).click();await p.waitForFunction(prefix=>![...document.querySelectorAll('.record')].some(e=>e.textContent.includes(prefix)),prefix);await p.qaClose();
  });
  await test('ordinary user cannot view admin and expired token is cleared',async()=>{
    const p=await pageFor(fixture.learner);await p.goto(base+'/admin');await text(p,'没有管理权限');await p.evaluate(()=>localStorage.setItem('colang.token','expired.invalid.token'));await p.goto(base+'/me');await text(p,'登录后继续学习');assert.equal(await p.evaluate(()=>localStorage.getItem('colang.token')),null);await p.qaClose();
  });
  console.log(`${count} browser regression scenarios passed.`);
})().catch(error=>{console.error(error);process.exitCode=1;}).finally(async()=>{await browser?.close();});
