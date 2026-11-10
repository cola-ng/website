// Dictionary parity with the former React UI, using the actual WASM build.
const {chromium}=require(process.env.PLAYWRIGHT_MODULE||'playwright');
const {fixture}=require('./ui-fixtures.cjs');
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path');
const {execFileSync}=require('node:child_process');
const target=JSON.parse(execFileSync('cargo',['metadata','--no-deps','--format-version','1'],{cwd:path.resolve(__dirname,'..'),encoding:'utf8'})).target_directory;
const output=path.join(target,'qa-dictionary');fs.mkdirSync(output,{recursive:true});
const base=process.env.QA_WEB||'http://127.0.0.1:5173';
assert(['127.0.0.1','localhost'].includes(new URL(base).hostname));
let browser,page;
(async()=>{
 browser=await chromium.launch({headless:true});
 const context=await browser.newContext({viewport:{width:1280,height:900}});
 const errors=[];let saved=false,toggles=0,fail=false,vocabulary=0;
 let history=[{id:1,word:'godless'},{id:2,word:'fast'}];
 await context.addInitScript(()=>localStorage.setItem('colang.token','dictionary-fixture'));
 await context.route('**/api/**',async route=>{
  const url=new URL(route.request().url()),method=route.request().method();let data=fixture(url,method);
  if(url.pathname==='/api/dict/lookup')data={word:{id:51,word:url.searchParams.get('word')},definitions:[{language:'zh',part_of_speech:'adj.',is_summary:true,definition:'无耻的'},{language:'zh',part_of_speech:'adj.',is_summary:false,definition:'不信上帝的'},{language:'en',definition:'Without moral principles.'}],relations:[{relation_type:'synonym',related_word:'shameless'}],pronunciations:[{id:1,dialect:'UK',ipa:'ɡɒdləs'}]};
  if(url.pathname==='/api/learn/favorites/check'){assert.deepEqual(route.request().postDataJSON(),{entity:'word',record_ids:[51]});data=saved?[51]:[];}
  if(url.pathname==='/api/learn/favorites/toggle'){
   toggles++;assert.deepEqual(route.request().postDataJSON(),{entity:'word',record_id:51});
   if(fail)return route.fulfill({status:503,json:{message:'暂时无法收藏'}});
   await new Promise(r=>setTimeout(r,150));saved=!saved;data={is_favorited:saved};
  }
  if(url.pathname.startsWith('/api/learn/vocabulary'))vocabulary++;
  if(url.pathname.startsWith('/api/dict/words/searched')){
   if(method==='DELETE')history=url.pathname.endsWith('/searched')?[]:history.filter(v=>v.id!==Number(url.pathname.split('/').at(-1)));
   data=method==='GET'?history:{};
  }
  return route.fulfill({json:data});
 });
 page=await context.newPage();page.on('pageerror',e=>errors.push(e.message));page.on('dialog',d=>d.accept());
 await page.goto(base+'/dict/godless');
 const star=page.getByRole('button',{name:'收藏',exact:true});await star.waitFor();await star.waitFor({state:'visible'});
 await page.waitForFunction(()=>document.querySelector('.word-favorite')?.disabled===false);
 assert.equal(await page.getByRole('button',{name:/加入.*生词本|收藏 \/|朗读/}).count(),0);
 assert.equal(await star.getAttribute('aria-pressed'),'false');
 await star.click();await page.getByRole('button',{name:'取消收藏',exact:true}).waitFor();assert(saved);
 await page.reload();await page.getByRole('button',{name:'取消收藏',exact:true}).waitFor();
 assert.equal(await page.getByRole('button',{name:'取消收藏',exact:true}).getAttribute('aria-pressed'),'true');
 await page.getByRole('button',{name:'取消收藏',exact:true}).focus();await page.keyboard.press('Enter');await page.getByRole('button',{name:'收藏',exact:true}).waitFor();assert(!saved);
 fail=true;await page.getByRole('button',{name:'收藏',exact:true}).click();await page.getByRole('alert').getByText('暂时无法收藏',{exact:true}).waitFor();assert.equal(await page.locator('.word-favorite').getAttribute('aria-pressed'),'false');fail=false;
 assert.equal(await page.getByText('不信上帝的',{exact:true}).count(),0);await page.getByRole('button',{name:'展开详细释义 (1)',exact:true}).click();await page.getByText('不信上帝的',{exact:true}).waitFor();await page.getByRole('button',{name:'收起详细释义',exact:true}).click();await page.getByText('不信上帝的',{exact:true}).waitFor({state:'detached'});assert.equal(await page.getByText('不信上帝的',{exact:true}).count(),0);
 const historyBox=page.locator('.dictionary-history');await historyBox.getByRole('button',{name:'删除',exact:true}).first().click();await historyBox.getByRole('link',{name:'godless',exact:true}).waitFor({state:'detached'});assert.equal(history.length,1);
 await historyBox.getByRole('button',{name:'清空',exact:true}).click();await historyBox.getByText('查过的单词会保存在这里。',{exact:true}).waitFor();assert.equal(history.length,0);
 await page.getByRole('button',{name:'收藏',exact:true}).click();await page.getByRole('button',{name:'取消收藏',exact:true}).waitFor();
 history=[{id:1,word:'godless'},{id:2,word:'fast'}];await page.reload();await page.getByRole('button',{name:'取消收藏',exact:true}).waitFor();
 for(const width of [1280,390]){
  await page.setViewportSize({width,height:900});
  await page.getByRole('tab',{name:'中文释义',exact:true}).click();await page.getByText('无耻的',{exact:true}).waitFor();
  await page.getByRole('tab',{name:'中文释义',exact:true}).focus();await page.keyboard.press('ArrowRight');await page.getByText('Without moral principles.',{exact:true}).waitFor();
  assert.equal(await page.getByRole('heading',{name:'基本释义',exact:true}).count(),0);
  const bounds=await page.locator('.word-title').evaluate(e=>{const a=e.querySelector('h2').getBoundingClientRect(),b=e.querySelector('button').getBoundingClientRect();return {sameRow:Math.abs(a.top-b.top)<12,gap:b.left-a.right,overflow:document.documentElement.scrollWidth>innerWidth+1};});
  assert(bounds.sameRow&&bounds.gap<=12&&!bounds.overflow);
  await page.getByRole('tab',{name:'中文释义',exact:true}).click();await page.getByRole('heading',{name:'godless',exact:true}).click();
  await page.screenshot({path:path.join(output,`dictionary-${width}.png`),fullPage:true});
 }
 await page.evaluate(()=>localStorage.removeItem('colang.token'));
 await page.getByRole('button',{name:'取消收藏',exact:true}).click();
 await page.waitForURL('**/login?next=%2Fdict%2Fgodless');
 assert.equal(vocabulary,0);assert.equal(toggles,4);assert.deepEqual(errors,[]);
 const result={favoriteStateAndPersistence:true,keyboard:true,errorRecovery:true,deleteAndClear:true,desktopAndMobile:true,vocabularyRequests:0};
 fs.writeFileSync(path.join(output,'report.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result));
})().catch(async e=>{if(page)await page.screenshot({path:path.join(output,'failure.png'),fullPage:true}).catch(()=>{});console.error(e);process.exitCode=1;}).finally(async()=>browser?.close());
