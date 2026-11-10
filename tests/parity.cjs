// Compare the frozen pre-migration UI with the actual Glory/WASM build.
// Both previews receive the same deterministic data in disposable contexts.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const {fixture,user}=require('./ui-fixtures.cjs');
const fs=require('node:fs');
const path=require('node:path');
const {execFileSync}=require('node:child_process');
const assert=require('node:assert/strict');
const target=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{encoding:'utf8',cwd:path.resolve(__dirname,'..')})).target_directory;
const output=path.join(target,'qa-parity');fs.mkdirSync(output,{recursive:true});
const settle=page=>page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
let checks=0;const results=[];
async function restored(page,name,width,base,requests){
 const verify=(condition,description)=>{assert(condition,`${name}@${width}: ${description}`);checks++;results.push({page:name,width,check:description});};
 verify(await page.locator('main').innerText().then(s=>s.trim().length>0),'page content renders');
 verify(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),'no page-wide horizontal overflow');
 if(name==='home'){
  for(const label of ['学习数据','今日任务','角色扮演','学习洞察'])verify(await page.getByRole('heading',{name:label,exact:true}).isVisible(),label);
  if(width===1280){const a=await page.locator('.dashboard-primary').boundingBox(),b=await page.locator('.dashboard-secondary').boundingBox();verify(b.x>a.x+a.width,'dashboard keeps the task sidebar');}
  await page.getByRole('button',{name:'周',exact:true}).click();await settle(page);verify(await page.locator('.study-column').count()===8,'eight real weekly buckets');
  await page.getByRole('button',{name:'月',exact:true}).click();await settle(page);verify(await page.locator('.study-column').count()===6,'six real monthly buckets');
 }else if(name==='profile'){
  verify(await page.getByRole('tab').count()===3,'profile / achievements / settings tabs');
  await page.getByRole('tab',{name:'成就',exact:true}).click();await settle(page);verify(await page.getByRole('heading',{name:'我的成就',exact:true}).isVisible(),'achievement panel');
  await page.keyboard.press('ArrowRight');await settle(page);verify(await page.getByRole('heading',{name:'AI 服务配置',exact:true}).isVisible(),'keyboard tab navigation');
  verify(await page.getByRole('textbox',{name:'API Key',exact:true}).count()===0,'provider editor stays closed until requested');
  await page.getByRole('button',{name:'添加 AI 配置',exact:true}).click();await settle(page);verify(await page.getByRole('dialog').getByRole('textbox',{name:'API Key',exact:true}).isVisible(),'provider editor opens');
  await page.keyboard.press('Escape');await settle(page);verify(await page.getByRole('dialog').count()===0,'Escape closes editor');
  await page.getByRole('tab',{name:'个人资料',exact:true}).click();await settle(page);await page.goto(base+'/me#settings');await page.getByRole('heading',{name:'AI 服务配置',exact:true}).waitFor();verify(await page.getByRole('tab',{name:'设置',exact:true}).getAttribute('aria-selected')==='true','same-document settings deep link changes the active tab');
 }else if(name==='dictionary'){
  verify(await page.locator('.dictionary-history').isVisible(),'search history sidebar');
  await page.getByRole('heading',{name:'使用例句',exact:true}).waitFor();await settle(page);verify(await page.getByText('Hello world.',{exact:false}).isVisible(),'word section navigation');
 }else if(name==='chat'){
  const feed=await page.locator('.chat-messages').boundingBox(),compose=await page.locator('.chat-composer').boundingBox();
  verify(feed.height>100&&compose.y>=feed.y+feed.height-1,'separate message viewport and composer');
  await page.getByRole('button',{name:'AI 中',exact:true}).click();await settle(page);verify(!await page.locator('.ai-message .message-zh').isVisible(),'language preference applies to messages');
  await page.reload();await page.getByRole('button',{name:'AI 中',exact:true}).waitFor();verify(await page.getByRole('button',{name:'AI 中',exact:true}).getAttribute('aria-pressed')==='false','language preference survives reload');
  await page.getByRole('button',{name:'AI 中',exact:true}).click();await settle(page);
  await page.getByRole('button',{name:'修改标题',exact:true}).click();await settle(page);verify(await page.getByRole('textbox',{name:'对话标题',exact:true}).inputValue()==='英语对话练习','rename opens with the existing title');await page.keyboard.press('Escape');await settle(page);
 }else if(name==='review'){
  verify(await page.locator('.review-mode').count()===5,'five review modes');
  await page.locator('a[href="/review/session"]').click();await settle(page);await page.getByRole('button',{name:'跳过此项',exact:true}).waitFor();
  verify(requests.some(u=>u.pathname==='/api/learn/review/items'&&!u.searchParams.has('type')),'smart review requests mixed recommendations');
  await page.getByRole('button',{name:'跳过此项',exact:true}).click();await settle(page);verify(await page.getByRole('heading',{name:'听音拼写',exact:true}).isVisible(),'mixed review switches the practice type per item');
 }else if(name==='read'){
  await page.getByRole('button',{name:'选择跟读主题',exact:true}).click();await settle(page);verify(await page.getByRole('dialog').getByRole('button',{name:'日常跟读',exact:true}).isVisible(),'subject picker');
  await page.getByRole('dialog').getByRole('button',{name:'日常跟读',exact:true}).click();await settle(page);verify(await page.getByRole('dialog').count()===0,'selecting a subject closes the picker');
 }else if(name==='admin'){
  verify(await page.getByRole('table').count()===1,'dictionary table');verify(await page.getByRole('dialog').count()===0,'no permanent editor below the list');
  await page.getByRole('button',{name:'新建记录',exact:true}).click();await settle(page);verify(await page.getByRole('dialog').getByRole('textbox',{name:'单词 *',exact:true}).isVisible(),'record editor opens');await page.keyboard.press('Escape');await settle(page);
 }
 if(width===390&&name==='home'){
  await page.getByRole('button',{name:'打开导航菜单',exact:true}).click();await settle(page);verify(await page.getByRole('dialog').getByRole('link',{name:'天天唠嗑',exact:true}).isVisible(),'mobile navigation drawer');await page.keyboard.press('Escape');await settle(page);
 }
}
(async()=>{
 const browser=await chromium.launch({headless:true});
 try {
  for(const [version,base] of [['react','http://127.0.0.1:5180'],['glory',process.env.QA_WEB||'http://127.0.0.1:5178']]){
   if(process.env.PARITY_VERSION && process.env.PARITY_VERSION!==version)continue;
   for(const width of [1280,390]){
    const context=await browser.newContext({viewport:{width,height:900}});
    await context.addInitScript(user=>{if(!sessionStorage.getItem('qa-seeded')){localStorage.setItem('colang.token','ui-fixture');localStorage.setItem('colang.user',JSON.stringify(user));sessionStorage.setItem('qa-seeded','1');}},user);
    const requests=[];
    await context.route('**/api/**',async route=>{const url=new URL(route.request().url());requests.push(url);const body=fixture(url,route.request().method());await route.fulfill({json:body});});
    const page=await context.newPage();
    for(const [name,url] of [['home','/'],['profile','/me'],['dictionary','/dict/hello'],['translation','/translate'],['chat','/chat/1'],['review','/review'],['read','/read'],['stages','/stages'],['admin','/admin/dict']]){
     const errors=[];const onerror=e=>errors.push(e.message);page.on('pageerror',onerror);
     const onconsole=m=>{if(m.type()==='error')errors.push(m.text());};page.on('console',onconsole);
     await page.goto(base+url);await page.waitForTimeout(1200);
     await page.screenshot({path:path.join(output,`${version}-${name}-${width}.png`),fullPage:true});
     fs.writeFileSync(path.join(output,`${version}-${name}-${width}.json`),JSON.stringify({errors,text:await page.locator('body').innerText()},null,2));
     if(version==='glory'){assert.deepEqual(errors,[],`${name}@${width}`);await restored(page,name,width,base,requests);}
     page.off('pageerror',onerror);
     page.off('console',onconsole);
    }
    if(version==='glory'&&width===1280){
     await page.evaluate(()=>localStorage.removeItem('colang.token'));await page.goto(base+'/dict');
     await page.getByRole('textbox',{name:'单词或短语',exact:true}).fill('hello');await page.keyboard.press('Enter');await page.getByRole('tab',{name:'中文释义',exact:true}).waitFor();
     await page.reload();await page.getByRole('tab',{name:'中文释义',exact:true}).waitFor();await page.locator('.dictionary-history').getByRole('link',{name:'hello',exact:true}).waitFor();checks++;results.push({page:'dictionary',width,check:'anonymous Enter lookup and persisted search history'});
     await page.locator('.dictionary-history').getByRole('button',{name:'删除',exact:true}).click();await page.locator('.dictionary-history').getByRole('link',{name:'hello',exact:true}).waitFor({state:'detached'});assert.equal(await page.locator('.dictionary-history').getByRole('link',{name:'hello',exact:true}).count(),0);checks++;results.push({page:'dictionary',width,check:'anonymous history deletion'});
    }
    await context.close();
   }
  }
  fs.writeFileSync(path.join(output,'report.json'),JSON.stringify({reference:'b486031a',checks,results},null,2));
  console.log(`${checks} restored-layout and interaction checks passed.`);
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});
