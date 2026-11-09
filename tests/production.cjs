// Public page checks and disposable OAuth start/cancel sessions on production.
// Optional conversation checks use only an explicitly provisioned QA account.
// This browser has a fresh profile and never reads the user's browser credentials.
const { chromium } = require(process.env.PLAYWRIGHT_MODULE || 'playwright');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');
const base = process.env.PRODUCTION_BASE || 'https://cola.ng';
assert.equal(new URL(base).origin, 'https://cola.ng');
const artifacts = JSON.parse(execFileSync('cargo', ['metadata','--no-deps','--format-version','1'], {encoding:'utf8'})).target_directory;
const output = path.join(artifacts,'qa-production'); fs.mkdirSync(output,{recursive:true});
let browser;
(async()=>{
  browser=await chromium.launch({headless:true});
  const context=await browser.newContext();
  const api=context.request;
  const providers=await api.get(base+'/api/auth/providers');
  assert.equal(providers.status(),200);
  assert.deepEqual((await providers.json()).providers.map(p=>p.id).sort(),['github','google']);
  assert.equal((await api.get(base+'/api/subscription/plans')).status(),200);
  for(const [provider,host] of [['github','github.com'],['google','accounts.google.com']]){
    const start=await api.get(base+`/api/auth/${provider}/start?next=%2Fme`,{maxRedirects:0});
    assert.equal(start.status(),302);
    const url=new URL(start.headers().location);
    assert.equal(url.hostname,host);
    assert.equal(url.searchParams.get('redirect_uri'),base+`/api/auth/${provider}/callback`);
    assert.equal(url.searchParams.get('code_challenge_method'),'S256');
    const cookie=start.headers()['set-cookie'];
    for(const flag of ['HttpOnly','Secure','SameSite=Lax'])assert(cookie.includes(flag));
    const cancel=await api.get(base+`/api/auth/${provider}/callback?state=${url.searchParams.get('state')}&error=access_denied`,{maxRedirects:0});
    assert.equal(cancel.status(),302);
    assert.equal(cancel.headers().location,'/login?oauth_error=access_denied');
  }
  const page=await context.newPage();
  let checkingPublic=true;const publicApiFailures=[];
  page.on('response',response=>{const url=new URL(response.url());if(checkingPublic&&url.origin===base&&url.pathname.startsWith('/api/')&&response.status()>=400)publicApiFailures.push({path:url.pathname,status:response.status()});});
  const errors=[];page.on('pageerror',error=>{errors.push(error.message);console.error('PAGE ERROR',page.url(),error.message);});
  page.on('console',message=>{if(message.type()==='error')console.error('CONSOLE',page.url(),message.text());});
  page.on('requestfailed',request=>console.error('REQUEST FAILED',request.url(),request.failure()?.errorText));
  const routes=['/','/login','/auth/complete','/dict','/dict/hello','/translate','/stages','/subscription','/terms','/privacy','/review','/me'];
  let audits=0;
  for(const width of [1280,390]){
    await page.setViewportSize({width,height:900});
    for(const route of routes){
      console.log('AUDIT',route,width);
      const response=await page.goto(base+route);
      assert.equal(response.status(),200,route);
      await page.locator('main h1,main h2').first().waitFor({timeout:180000}).catch(async error=>{await page.screenshot({path:path.join(output,'failure.png'),fullPage:true});throw new Error(route+'@'+width+': '+error.message+'; page errors: '+errors.join('; '));});
      await page.waitForFunction(()=>!document.querySelector('main .loading-status'));
      assert(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),route+' overflow');
      if(['/login','/review','/me'].includes(route)){
        assert(await page.getByRole('button',{name:'使用 GitHub',exact:true}).isVisible());
        assert(await page.getByRole('button',{name:'使用 Google',exact:true}).isVisible());
      }
      audits++;
    }
    await page.goto(base+'/login');
    await page.getByRole('button',{name:'使用 Google',exact:true}).waitFor();
    await page.screenshot({path:path.join(output,`login-${width}.png`),fullPage:true});
  }
  checkingPublic=false;assert.deepEqual(publicApiFailures,[]);
  let conversation,accountPolicy;
  if(process.env.PRODUCTION_QA_FIXTURE){
    const qa=JSON.parse(fs.readFileSync(process.env.PRODUCTION_QA_FIXTURE,'utf8'));
    assert(/^qa-stream-\d+@example\.invalid$/.test(qa.email),'Use only a disposable production QA account');
    await page.setViewportSize({width:1280,height:900});await page.goto(base+'/login');
    await page.getByRole('textbox',{name:'邮箱',exact:true}).fill(qa.email);
    await page.getByRole('textbox',{name:'密码',exact:true}).fill(qa.password);
    await page.getByRole('button',{name:'登录',exact:true}).last().click();await page.waitForURL('**/me');
    await page.getByRole('heading',{name:'我的学习',exact:true}).waitFor();
    if(qa.expect_personal_required){
      page.on('dialog',dialog=>dialog.accept());
      const handle='qaprod_'+Date.now();
      await page.getByRole('textbox',{name:'Handle（用户标识）',exact:true}).fill(handle);await page.getByRole('button',{name:'保存资料',exact:true}).click();await page.getByText('资料已保存',{exact:true}).waitFor();
      await page.reload();await page.getByRole('textbox',{name:'Handle（用户标识）',exact:true}).waitFor();assert.equal(await page.getByRole('textbox',{name:'Handle（用户标识）',exact:true}).inputValue(),handle);
      const chooser=page.waitForEvent('filechooser');await page.getByRole('button',{name:'上传头像（居中裁剪）',exact:true}).click();await(await chooser).setFiles(path.resolve(__dirname,'../front/public/colang-logo.png'));
      await page.waitForFunction(()=>document.querySelector('.profile-avatar')?.src.startsWith('data:image/'));await page.reload();await page.waitForFunction(()=>document.querySelector('.profile-avatar')?.src.startsWith('data:image/'));
      await page.getByRole('button',{name:'删除头像',exact:true}).click();await page.waitForFunction(()=>document.querySelector('.profile-avatar')?.src.endsWith('/colang-logo.svg'));
      await page.getByRole('tab',{name:'设置',exact:true}).click();await page.getByRole('button',{name:'添加 AI 配置',exact:true}).click();await page.getByRole('textbox',{name:'API Key',exact:true}).fill('qa-production-invalid-key');await page.getByRole('button',{name:'添加并启用',exact:true}).click();await page.locator('.ai-access-status').getByText(/当前使用你自己的/).waitFor();
      assert.equal(await page.getByRole('textbox',{name:'API Key',exact:true}).count(),0);
      await page.locator('.provider-row').getByRole('button',{name:'编辑',exact:true}).click();await page.getByRole('textbox',{name:'对话模型（可选）',exact:true}).fill('glm-4-flash');await page.getByRole('button',{name:'保存配置',exact:true}).click();await page.getByRole('button',{name:'保存配置',exact:true}).waitFor({state:'detached'});
      await page.locator('.provider-row').getByRole('button',{name:'删除配置',exact:true}).click();await page.locator('.provider-row').waitFor({state:'detached'});await page.locator('.ai-access-status').getByText(/请添加并启用自己的/).waitFor();
      await page.goto(base+'/chat/'+qa.chat_id);await page.getByRole('textbox',{name:'输入消息',exact:true}).fill('Hello, this is a free account.');await page.getByRole('button',{name:'发送消息',exact:true}).click();
      await page.getByRole('alert').getByText(/请先添加并启用自己的/).waitFor();assert.equal(await page.getByRole('textbox',{name:'输入消息',exact:true}).inputValue(),'Hello, this is a free account.');
      accountPolicy={handlePersisted:true,avatarUploadAndDelete:true,personalProviderCreateEditDelete:true,freePlatformDenied:true,draftPreserved:true};
      await page.screenshot({path:path.join(output,'personal-provider-required.png'),fullPage:true});
      console.log('Production account policy passed:',JSON.stringify(accountPolicy));
    }else{
    await page.goto(base+'/chat/'+qa.chat_id);
    await page.getByRole('textbox',{name:'输入消息',exact:true}).fill('I enjoy learning English. Ask me one short question.');
    const started=Date.now();
    await page.getByRole('button',{name:'发送消息',exact:true}).click();
    await page.waitForFunction(()=>document.querySelector('.streaming-reply p')?.textContent.trim().length>0,{},{timeout:95000});
    const partial=Date.now()-started;assert(await page.getByRole('button',{name:'发送消息',exact:true}).isDisabled());
    await page.locator('.streaming-reply').waitFor({state:'detached',timeout:95000});
    assert(await page.getByRole('button',{name:'发送消息',exact:true}).isEnabled());assert.equal(await page.getByRole('alert').count(),0);
    const elapsed=Date.now()-started;assert(partial<elapsed);
    const response=await api.post(base+'/api/login',{data:{email:qa.email,password:qa.password}});assert.equal(response.status(),200);
    const token=(await response.json()).access_token;
    const history=await api.get(base+`/api/learn/chats/${qa.chat_id}/turns?from_latest=true`,{headers:{Authorization:'Bearer '+token}});assert.equal(history.status(),200);
    const turns=(await history.json()).items;assert(turns.filter(t=>t.speaker==='assistant'&&t.status==='completed').length>=2);
    conversation={first_visible_ms:partial,complete_ms:elapsed,persisted:true};
    await page.screenshot({path:path.join(output,'conversation.png'),fullPage:true});
    console.log('Production real conversation passed:',JSON.stringify(conversation));
    }
    await page.getByRole('button',{name:/我的账户$/}).click();await page.getByRole('button',{name:'退出登录',exact:true}).click();
  }
  assert.deepEqual(errors,[]);
  for(const name of ['failure.png']){const file=path.join(output,name);if(fs.existsSync(file))fs.unlinkSync(file);}
  fs.writeFileSync(path.join(output,'report.json'),JSON.stringify({origin:base,audits,providers:['github','google'],secureCookies:true,pkce:true,cancellation:true,conversation,accountPolicy,publicApiFailures,browserErrors:errors,passed:true},null,2));
  console.log(`Production passed: ${audits} page/viewport checks, both provider redirects, secure cookies, PKCE and cancellation.`);
})().catch(error=>{console.error(error);process.exitCode=1;}).finally(async()=>{await browser?.close();});
