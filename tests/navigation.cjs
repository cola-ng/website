// Actual WASM navigation: document identity and request counts catch hard reloads.
const {chromium} = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const {fixture,user} = require('./ui-fixtures.cjs');
const base = process.env.QA_WEB || 'http://127.0.0.1:5173';
const authenticated = process.env.QA_NAV_PUBLIC !== '1';
let browser;
(async()=>{
  browser=await chromium.launch({headless:true});
  const context=await browser.newContext({viewport:{width:1280,height:900}});
  const page=await context.newPage(), errors=[], documents=[];
  page.on('pageerror',e=>errors.push(e.message));
  page.on('request',r=>{if(r.isNavigationRequest()&&r.frame()===page.mainFrame())documents.push(r.url());});
  if(authenticated){
    assert(['localhost','127.0.0.1'].includes(new URL(base).hostname));
    await context.route('**/api/**',r=>{
      const url=new URL(r.request().url()),data=fixture(url,r.request().method());
      if(url.pathname==='/api/dict/lookup')data.word={...data.word,word:url.searchParams.get('word')};
      return r.fulfill({json:data});
    });
    await page.addInitScript(()=>localStorage.setItem('colang.token','navigation-fixture'));
  }
  await page.goto(base+'/dict');
  await page.getByRole('heading',{name:'词典翻译',exact:true}).waitFor();
  await page.evaluate(()=>window.navigationIdentity={});
  const initialDocuments=documents.length;
  async function unchanged(){assert(await page.evaluate(()=>!!window.navigationIdentity),'document was replaced');assert.equal(documents.length,initialDocuments);}
  async function destination(path,label,container=page.locator('.desktop-navigation')){
    await container.getByRole('link',{name:label,exact:true}).click();
    await page.waitForURL(base+path);
    await page.locator('main h1,main h2').first().waitFor();
    await page.waitForFunction(()=>!document.querySelector('main .loading-status'));
    await unchanged();
  }
  for(const [path,label] of [['/chat','天天唠嗑'],['/review','温故知新'],['/stages','角色扮演'],['/read','大声跟读'],['/dict','词典翻译']]){
    await destination(path,label);
    assert.equal(await page.locator('.desktop-navigation a[aria-current=page]').textContent(),label);
  }
  await page.goBack();await page.waitForURL(base+'/read');await unchanged();
  await page.goForward();await page.waitForURL(base+'/dict');await unchanged();
  const review=page.locator('.desktop-navigation').getByRole('link',{name:'温故知新',exact:true});
  await review.focus();await page.keyboard.press('Enter');await page.waitForURL(base+'/review');await unchanged();
  // Ctrl-click retains native new-tab behavior and leaves this document in place.
  const popupPromise=context.waitForEvent('page');
  await page.locator('.desktop-navigation').getByRole('link',{name:'词典翻译',exact:true}).click({modifiers:['Control']});
  const popup=await popupPromise;await popup.waitForURL(base+'/dict');await popup.close();
  assert.equal(new URL(page.url()).pathname,'/review');await unchanged();
  await page.setViewportSize({width:390,height:844});
  await page.getByRole('button',{name:'打开导航菜单',exact:true}).click();
  await destination('/stages','角色扮演',page.getByRole('dialog'));
  assert.equal(await page.getByRole('dialog').count(),0);
  await destination('/privacy','隐私政策',page.locator('footer'));await unchanged();
  if(authenticated){
    await page.setViewportSize({width:1280,height:900});
    await destination('/dict','词典翻译');
    await page.getByRole('textbox',{name:'单词或短语',exact:true}).fill('hello');
    await page.getByRole('button',{name:'查询',exact:true}).click();
    await page.waitForURL(base+'/dict/hello');await page.getByRole('heading',{name:'hello',exact:true}).waitFor();await unchanged();
    await destination('/dict','词典翻译');
    await page.getByRole('heading',{name:'探索每一个单词',exact:true}).waitFor();
    await page.goBack();await page.waitForURL(base+'/dict/hello');await page.getByRole('heading',{name:'hello',exact:true}).waitFor();await unchanged();
    for(const [word,path] of [['ice cream','ice%20cream'],['C++','C%2B%2B']]){
      await page.getByRole('textbox',{name:'单词或短语',exact:true}).fill(word);await page.getByRole('button',{name:'查询',exact:true}).click();
      await page.waitForURL(base+'/dict/'+path);await page.getByRole('heading',{name:word,exact:true}).waitFor();await unchanged();
      const direct=await context.newPage();await direct.goto(base+'/dict/'+path);await direct.getByRole('heading',{name:word,exact:true}).waitFor();await direct.close();
    }
    await page.getByRole('button',{name:'界面复检账户 · 我的账户',exact:true}).click();
    await page.getByRole('link',{name:'AI 服务设置',exact:true}).click();
    await page.waitForURL(base+'/me#settings');await page.getByRole('button',{name:'添加 AI 配置',exact:true}).waitFor();await unchanged();
    await page.getByRole('button',{name:'界面复检账户 · 我的账户',exact:true}).click();
    await page.getByRole('link',{name:'成就',exact:true}).click();await page.waitForURL(base+'/me#achievements');await unchanged();
    await page.goBack();await page.waitForURL(base+'/me#settings');await page.getByRole('button',{name:'添加 AI 配置',exact:true}).waitFor();await unchanged();
    await page.getByRole('button',{name:'界面复检账户 · 我的账户',exact:true}).click();
    await page.getByRole('button',{name:'退出登录',exact:true}).click();await page.waitForURL(base+'/');await unchanged();
    await destination('/review','温故知新');await page.getByRole('heading',{name:'登录后继续学习',exact:true}).waitFor();
    await page.route('**/api/login',r=>r.fulfill({json:{user,is_admin:true,access_token:'navigation-login'}}));
    await page.getByRole('textbox',{name:'邮箱',exact:true}).fill('ui@example.test');
    await page.getByRole('textbox',{name:'密码',exact:true}).fill('valid-password');
    await page.getByRole('button',{name:'登录',exact:true}).last().click();
    await page.getByRole('heading',{name:'温故知新',exact:true}).waitFor();await unchanged();
    assert.equal(await page.evaluate(()=>localStorage.getItem('colang.token')),'navigation-login');
  }
  assert.deepEqual(errors,[]);
  console.log(JSON.stringify({navigation:'passed',authenticated,documentRequests:documents.length,desktop:true,mobile:true,history:true,keyboard:true,modifiedClick:true}));
})().catch(e=>{console.error(e);process.exitCode=1;}).finally(async()=>{await browser?.close();});
