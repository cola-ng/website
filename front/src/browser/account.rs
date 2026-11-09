use super::*;

pub fn login_page(required: bool) -> impl Widget {
    widget(move |scope| {
        let mode = scope.cage("login".to_string());
        let email = scope.cage(String::new());
        let pass = scope.cage(String::new());
        let name = scope.cage(String::new());
        let phone = scope.cage(String::new());
        let repeat = scope.cage(String::new());
        let agreed = scope.cage(false);
        let attempted = scope.cage(false);
        let action = Resource::new(scope);
        scope.effect(move || {
            mode.get();
            attempted.revise(|v| *v = false);
            action.error.revise(String::clear);
        });
        view().class("panel auth-card").fill((
            title(if required { "登录后继续学习" } else { "欢迎来到开朗英语" }),
            choices_enabled(mode,&[("login","登录"),("register","注册")],action.busy.map(|b|!*b).into()),
            field_state("邮箱",email,Purpose::Email,email.map(move |v|attempted.get()&&!v.trim().contains('@')).into(),action.busy.map(|b|!*b).into()),
            field_state("密码",pass,Purpose::Password,pass.map(move |v|attempted.get()&&(v.is_empty()||(mode.get()=="register"&&v.len()<8))).into(),action.busy.map(|b|!*b).into()),
            when(mode.map(|v|v=="register"),move || view().class("form-fields").fill((
                field_state("昵称",name,Purpose::Text,name.map(move |v|attempted.get()&&v.trim().is_empty()).into(),action.busy.map(|b|!*b).into()),
                field_state("手机（可选）",phone,Purpose::Tel,false.into(),action.busy.map(|b|!*b).into()),
                field_state("再次输入密码",repeat,Purpose::Password,repeat.map(move |v|attempted.get()&&v!=&pass.get()).into(),action.busy.map(|b|!*b).into()),
                checkbox_control().label("我已阅读并同意服务条款和隐私政策").checked(agreed).enabled(action.busy.map(|b|!*b)).on_checked_change(move |v|agreed.revise(|b|*b=v)),
                view().class("actions").fill((nav("服务条款","/terms"),nav("隐私政策","/privacy"))),
            ))),
            action_button().label(mode.map(|v|if v=="register"{"创建账户"}else{"登录"}.into())).loading(action.busy).on_press(move || {
                if action.busy.get(){return;}
                attempted.revise(|v|*v=true);
                let register=mode.get()=="register";
                let email=email.get().trim().to_lowercase();
                if !email.contains('@')||pass.get().is_empty(){action.error.revise(|e|*e="请输入有效邮箱和密码".into());return;}
                if register&&(pass.get().len()<8||pass.get()!=repeat.get()||name.get().trim().is_empty()||!agreed.get()){
                    action.error.revise(|e|*e="请填写昵称，输入至少 8 字节且一致的密码，并同意条款".into());return;
                }
                let body=if register{json!({"email":email,"password":pass.get(),"name":name.get().trim(),"phone":if phone.get().trim().is_empty(){Value::Null}else{json!(phone.get().trim())}})}else{json!({"email":email,"password":pass.get()})};
                action.run("POST",if register{"/api/register"}else{"/api/login"}.into(),Some(body),move |value|{
                    match save_auth(&value){
                        Ok(())=>{
                            let next=param("next").unwrap_or_else(||{
                                if window().location().pathname().unwrap_or_default()=="/auth"{format!("/auth{}",window().location().search().unwrap_or_default())}
                                else if required{format!("{}{}",window().location().pathname().unwrap_or_default(),window().location().search().unwrap_or_default())}
                                else{"/me".into()}
                            });
                            go(safe_return(&next));
                        },
                        Err(e)=>action.error.revise(|v|*v=e),
                    }
                });
            }),
            action.status(),
            p("第三方登录尚未配置，请使用邮箱登录。"),
        )).build(scope)
    })
}
pub fn authorize() -> impl Widget {
    widget(|scope| {
        if token().is_none() {
            return login_page(false).build(scope);
        }
        let action = Resource::new(scope);
        let status = scope.cage("已登录，可以继续使用网站。".to_string());
        let redirect = param("redirect_uri");
        let state = param("state");
        if let (Some(redirect), Some(state)) = (redirect, state) {
            if !desktop_callback(&redirect) || state.trim().is_empty() {
                action.error.revise(|e| {
                    *e = "桌面登录回调地址或状态无效，请从桌面应用重新发起登录。".into()
                });
            } else {
                status.revise(|s| *s = "正在完成桌面授权…".into());
                action.run(
                    "POST",
                    "/api/auth/code".into(),
                    Some(json!({"redirect_uri":redirect,"state":state})),
                    move |v| {
                        if let Ok(mut url) = url::Url::parse(&string(&v, "redirect_uri"))
                            && desktop_callback(url.as_str())
                        {
                            url.query_pairs_mut()
                                .append_pair("code", &string(&v, "code"))
                                .append_pair("state", &string(&v, "state"));
                            status.revise(|s| *s = "授权已完成，正在返回桌面应用。".into());
                            go(url.as_str());
                        }
                    },
                );
            }
        }
        view()
            .class("panel auth-card")
            .fill((
                title("登录授权"),
                text().content(status),
                action.status(),
                nav("继续学习", "/me"),
            ))
            .build(scope)
    })
}
pub fn profile(session: Cage<Value>) -> impl Widget {
    widget(move |scope| {
        let name = scope.cage(string(&session.get(), "name"));
        let phone = scope.cage(string(&session.get(), "phone"));
        let old_pass = scope.cage(String::new());
        let new_pass = scope.cage(String::new());
        let repeat = scope.cage(String::new());
        let action = Resource::new(scope);
        let notice = scope.cage(String::new());
        let account = Resource::new(scope);
        account.load("/api/me/account-status");
        let summary = Resource::new(scope);
        summary.load("/api/learn/summary");
        let study = Resource::new(scope);
        study.load("/api/learn/study-time?days=7");
        let achievements = Resource::new(scope);
        achievements.load("/api/achievements/my");
        let tasks = Resource::new(scope);
        tasks.load("/api/learn/daily");
        view().fill((title("我的学习"),view().class("cards").fill((
            view().class("panel").fill((heading("个人资料"),media::avatar(session),p(string(&session.get(),"email")),field_state("昵称",name,Purpose::Text,false.into(),action.busy.map(|b|!*b).into()),field_state("手机（可选）",phone,Purpose::Tel,false.into(),action.busy.map(|b|!*b).into()),
                action_button().label("保存资料").enabled(action.busy.map(|b|!*b)).on_press(move||action.run("PUT","/api/me".into(),Some(json!({"name":name.get().trim(),"phone":phone.get().trim()})),move|value|{let admin=session.get()["is_admin"].clone();let mut user=value;user["is_admin"]=admin;session.revise(|s|*s=user);notice.revise(|s|*s="资料已保存".into());})),
                action_button().label("上传头像（居中裁剪）").variant(sapid::ButtonVariant::Outline).enabled(action.busy.map(|b|!*b)).on_press(move||media::avatar_upload(action,session)),
                danger_button().label("删除头像").enabled(action.busy.map(|b|!*b)).on_press(move||{if confirm("删除当前头像？"){action.run("DELETE","/api/me/avatar".into(),None,move|v|{session.revise(|s|s["avatar"]=v["avatar"].clone());notice.revise(|s|*s="头像已删除".into());});}}),action.status(),success_notice(notice,action),
            )),
            view().class("panel").fill((heading("账户安全"),field_state("当前密码",old_pass,Purpose::Password,false.into(),action.busy.map(|b|!*b).into()),field_state("新密码",new_pass,Purpose::Password,false.into(),action.busy.map(|b|!*b).into()),field_state("确认新密码",repeat,Purpose::Password,false.into(),action.busy.map(|b|!*b).into()),
                action_button().label("更新密码").enabled(action.busy.map(|b|!*b)).on_press(move||{
                    if new_pass.get().len()<8 || new_pass.get()!=repeat.get(){action.error.revise(|e|*e="新密码至少 8 字节，两次输入必须一致".into());return;}
                    action.run("POST","/api/me/change-password".into(),Some(json!({"current_password":if old_pass.get().is_empty(){Value::Null}else{json!(old_pass.get())},"new_password":new_pass.get()})),move |_|{old_pass.revise(String::clear);new_pass.revise(String::clear);repeat.revise(String::clear);notice.revise(|s|*s="密码已更新".into());});
                }),account.status(),
                each(account.data.map(|v|items(&v["linked_identities"]))).key(|v|v.to_string()).seed(move|v|{let id=string(&v,"id");view().fill((p(format!("{} {}",string(&v,"provider"),string(&v,"email"))),danger_button().label("解除绑定").enabled(action.busy.map(|b|!*b)).on_press(move||{if confirm("解除该账户的绑定？"){action.run("DELETE",format!("/api/me/oauth/{id}"),None,move |_|account.load("/api/me/account-status"));}})))})
            )),
        )),
        data_section("学习概览",summary),data_section("最近 7 天学习时长",study),
        view().class("panel").fill((heading("今日任务"),tasks.status(),each(tasks.data.map(|v|items(&v["tasks"]))).key(|v|v.to_string()).seed(move|v|{
            let id=string(&v,"id");let completed=v["status"]=="completed";let target=v["target_value"].clone();
            view().class("history-row").fill((p(string(&v,"task_title")),checkbox_control().label("已完成").checked(completed).on_checked_change(move|next|action.run("PUT",format!("/api/learn/daily/tasks/{id}"),Some(json!({"status":if next{"completed"}else{"pending"},"current_value":if next{target.clone()}else{json!(0)}})),move |_|tasks.load("/api/learn/daily")))))
        }))),data_section("成就",achievements),providers(),view().class("actions").fill((nav("管理订阅和钱包","/subscription"),nav("生词复习","/review"))),
        )).build(scope)
    })
}
pub fn data_section(label_text: &'static str, resource: Resource) -> impl Widget {
    view().class("panel").fill((
        heading(label_text),
        resource.status(),
        each(resource.data.map(|v| {
            if v.is_null() {
                vec![]
            } else if v.is_array() {
                items(v)
            } else {
                vec![v.clone()]
            }
        }))
        .key(|v| v.to_string())
        .seed(|v| details(&v)),
    ))
}
fn providers() -> impl Widget {
    widget(|scope| {
        let providers = Resource::new(scope);
        providers.load("/api/ai-providers");
        let action = Resource::new(scope);
        let kind = scope.cage("zhipu".to_string());
        let key = scope.cage(String::new());
        let model = scope.cage(String::new());
        let app = scope.cage(String::new());
        let access = scope.cage(String::new());
        let editing = scope.cage(None::<i64>);
        let notice = scope.cage(String::new());
        view().class("panel").fill((heading("AI 服务配置"),providers.status(),choices_enabled(kind,&[("zhipu","智谱"),("doubao","豆包")],action.busy.map(|b|!*b).into()),field_state("API Key",key,Purpose::Password,false.into(),action.busy.map(|b|!*b).into()),field_state("对话模型（可选）",model,Purpose::Text,false.into(),action.busy.map(|b|!*b).into()),when(kind.map(|k|k=="doubao"),move||view().fill((field_state("App ID",app,Purpose::Text,false.into(),action.busy.map(|b|!*b).into()),field_state("Access Token",access,Purpose::Password,false.into(),action.busy.map(|b|!*b).into())))),
            action_button().label(editing.map(|v|if v.is_some(){"保存配置"}else{"添加配置"}.to_string())).enabled(action.busy.map(|b|!*b)).on_press(move||{
                if key.get().trim().is_empty(){action.error.revise(|e|*e="请输入 API Key；编辑时请重新填写密钥".into());return;}
                let mut settings=json!({"api_key":key.get().trim()});if !model.get().trim().is_empty(){settings["chat_model"]=json!(model.get().trim());}
if kind.get()=="doubao"{settings["app_id"]=json!(app.get().trim());settings["access_token"]=json!(access.get().trim());}
                let (method,path,body)=if let Some(id)=editing.get(){("PUT",format!("/api/ai-providers/{id}"),json!({"settings":settings}))}else{("POST","/api/ai-providers".into(),json!({"provider_type":kind.get(),"settings":settings,"set_active":false}))};
                action.run(method,path,Some(body),move |_|{key.revise(String::clear);access.revise(String::clear);editing.revise(|v|*v=None);providers.load("/api/ai-providers");notice.revise(|s|*s="配置已保存".into());});
            }),
            each(providers.data.map(items)).key(|v|v.to_string()).seed(move|v|{
                let id=v["id"].as_i64().unwrap_or_default();let original=v.clone();
                view().class("provider-row").fill((p(format!("{} · {}",string(&v,"provider_type"),if v["is_active"]==true{"使用中"}else{"未启用"})),
                    action_button().label("编辑").enabled(action.busy.map(|b|!*b)).variant(sapid::ButtonVariant::Outline).on_press(move||{editing.revise(|v|*v=Some(id));kind.revise(|k|*k=string(&original,"provider_type"));model.revise(|m|*m=string(&original["settings"],"chat_model"));key.revise(String::clear);access.revise(String::clear);app.revise(|s|*s=string(&original["settings"],"app_id"));}),
                    action_button().label("测试连接").enabled(action.busy.map(|b|!*b)).on_press(move||action.run("POST",format!("/api/ai-providers/{id}/test"),None,move|v|{if v["success"]==true{notice.revise(|s|*s="连接成功".into());}else{notice.revise(String::clear);action.error.revise(|e|*e=string(&v,"error"));}})),
                    action_button().label("启用").enabled(action.busy.map(|b|!*b)).on_press(move||action.run("POST",format!("/api/ai-providers/{id}/activate"),None,move |_|providers.load("/api/ai-providers"))),
                    danger_button().label("删除配置").enabled(action.busy.map(|b|!*b)).on_press(move||{if confirm("删除此 AI 配置？"){action.run("DELETE",format!("/api/ai-providers/{id}"),None,move |_|providers.load("/api/ai-providers"));}}),
                ))
            }),action.status(),success_notice(notice,action),
        )).build(scope)
    })
}
pub fn subscription() -> impl Widget {
    widget(|scope| {
        let plans = Resource::new(scope);
        plans.load("/api/subscription/plans");
        let current = Resource::new(scope);
        let wallet = Resource::new(scope);
        let transactions = Resource::new(scope);
        let action = Resource::new(scope);
        let trial = scope.cage(false);
        let notice = scope.cage(String::new());
        if token().is_some() {
            current.load("/api/subscription/current");
            wallet.load("/api/wallet");
            transactions.load("/api/wallet/transactions?per_page=30");
        }
        view()
            .fill((
                title("订阅与钱包"),
                plans.status(),
                checkbox_control()
                    .label("使用试用期（套餐支持时）")
                    .checked(trial)
                    .on_checked_change(move |v| trial.revise(|b| *b = v)),
                view().class("cards").fill(
                    each(plans.data.map(items))
                        .key(|v| v.to_string())
                        .seed(move |v| {
                            let code = string(&v, "code");
                            view().class("panel").fill((
                                heading(string(&v, "name_zh")),
                                p(string(&v, "description_zh")),
                                text()
                                    .class("plan-price")
                                    .content(string(&v, "price_display")),
                                nodes(
                                    items(&v["features"])
                                        .iter()
                                        .map(|feature| {
                                            boxed(p(feature.as_str().unwrap_or_default()))
                                        })
                                        .collect(),
                                ),
                                p(format!(
                                    "可试用 {} 天",
                                    v["trial_days"].as_i64().unwrap_or(0)
                                )),
                                action_button()
                                    .label("选择套餐")
                                    .enabled(action.busy.map(|b| !*b))
                                    .on_press(move || {
                                        if token().is_none() {
                                            go("/login?next=%2Fsubscription");
                                            return;
                                        }
                                        if !confirm("确认选择此套餐？") {
                                            return;
                                        }
                                        action.run(
                                            "POST",
                                            "/api/subscription/subscribe".into(),
                                            Some(json!({"plan_code":code,"use_trial":trial.get()})),
                                            move |v| {
                                                current.load("/api/subscription/current");
                                                wallet.load("/api/wallet");
                                                notice.revise(|n| {
                                                    *n = if v["payment_required"] == true {
                                                        format!(
                                                            "订单已创建，等待支付。订单号：{}",
                                                            string(&v, "order_no")
                                                        )
                                                    } else {
                                                        "订阅已开通".into()
                                                    }
                                                });
                                            },
                                        );
                                    }),
                            ))
                        }),
                ),
                action.status(),
                success_notice(notice, action),
                when(token().is_some(), move || {
                    view().fill((
                        view().class("panel").fill((
                            heading("当前订阅"),
                            current.status(),
                            each(
                                current
                                    .data
                                    .map(|v| if v.is_null() { vec![] } else { vec![v.clone()] }),
                            )
                            .key(|v| v.to_string())
                            .seed(|v| {
                                let s = &v["subscription"];
                                if v["has_subscription"] == true {
                                    nodes(vec![
                                        boxed(p(string(&s["plan"], "name_zh"))),
                                        boxed(p(format!(
                                            "状态：{} · 剩余 {} 天",
                                            string(s, "status"),
                                            string(&v, "days_remaining")
                                        ))),
                                        boxed(p(format!("到期时间：{}", string(s, "expires_at")))),
                                    ])
                                } else {
                                    nodes(vec![boxed(p("尚未开通订阅"))])
                                }
                            }),
                        )),
                        action_button()
                            .label("取消订阅")
                            .enabled(action.busy.map(|b| !*b))
                            .variant(sapid::ButtonVariant::Outline)
                            .on_press(move || {
                                if confirm("确认取消当前订阅？") {
                                    action.run(
                                        "POST",
                                        "/api/subscription/cancel".into(),
                                        Some(json!({"reason":"用户主动取消"})),
                                        move |_| current.load("/api/subscription/current"),
                                    );
                                }
                            }),
                        view().class("panel").fill((
                            heading("钱包"),
                            wallet.status(),
                            text().content(wallet.data.map(|v| {
                                format!(
                                    "可用余额 ¥{:.2}",
                                    v["account"]["balance"].as_i64().unwrap_or(0) as f64 / 100.0
                                )
                            })),
                        )),
                        view().class("panel").fill((
                            heading("交易记录"),
                            transactions.status(),
                            empty(transactions, "暂无交易记录。"),
                            each(transactions.data.map(items))
                                .key(|v| v.to_string())
                                .seed(|v| {
                                    p(format!(
                                        "{} · ¥{:.2} · {}",
                                        string(&v, "created_at"),
                                        v["amount"].as_i64().unwrap_or(0) as f64 / 100.0,
                                        string(&v, "description")
                                    ))
                                }),
                        )),
                    ))
                })
                .otherwise(|| nav("登录后查看订阅和钱包", "/login?next=%2Fsubscription")),
            ))
            .build(scope)
    })
}
