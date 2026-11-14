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
            social_buttons(None, action),
            when(param("oauth_error").is_some(), || view().role(Role::Alert).class("error").fill(p(oauth_error()))),
        )).build(scope)
    })
}
fn login_return() -> String {
    param("next").unwrap_or_else(|| {
        let path = window().location().pathname().unwrap_or_default();
        if path == "/auth" || Route::parse(&path).requires_auth() {
            format!("{path}{}", window().location().search().unwrap_or_default())
        } else {
            "/me".into()
        }
    })
}

fn oauth_error() -> &'static str {
    match param("oauth_error").as_deref() {
        Some("access_denied") => "已取消第三方登录，可以重试或使用邮箱登录。",
        Some("account_exists") => {
            "此邮箱已有账户。请先用邮箱和密码登录，再到「我的学习 → 账户安全」绑定第三方账户。"
        }
        Some("already_linked" | "provider_already_linked") => {
            "此第三方账户已绑定其他账户，或当前账户已有同类绑定。请在账户安全中检查。"
        }
        Some("verified_email_required") => "请先在第三方账户中验证邮箱，再重新登录。",
        Some("account_disabled") => "账户已停用或锁定，请联系管理员。",
        Some("invalid_state") => "登录会话已过期或浏览器不一致，请从本站重新发起登录。",
        Some("not_configured" | "unsupported_provider") => "该登录方式尚未配置，请使用邮箱登录。",
        _ => "第三方登录暂时未完成，请重试或使用邮箱登录。",
    }
}

fn social_buttons(account: Option<Resource>, action: Resource) -> impl Widget {
    widget(move |scope| {
        let config = Resource::new(scope);
        config.load("/api/auth/providers");
        view()
            .class("social-login")
            .fill((
                config.status(),
                each(scope.bond(move || {
                    let linked = account.map(|a| a.data.get());
                    if account.is_some_and(|a| a.busy.get() || !a.data.get().is_object()) {
                        return Vec::new();
                    }
                    items(&config.data.get()["providers"])
                        .into_iter()
                        .filter(|p| {
                            !linked.as_ref().is_some_and(|a| {
                                items(&a["linked_identities"])
                                    .iter()
                                    .any(|i| i["provider"] == p["id"] && i["verified"] == true)
                            })
                        })
                        .collect::<Vec<_>>()
                }))
                .key(|v| string(v, "id"))
                .seed(move |provider| {
                    let id = string(&provider, "id");
                    action_button()
                        .variant(sapid::ButtonVariant::Outline)
                        .label(format!(
                            "{} {}",
                            if account.is_some() {
                                "绑定"
                            } else {
                                "使用"
                            },
                            string(&provider, "name")
                        ))
                        .enabled(action.busy.map(|b| !*b))
                        .on_press(move || {
                            if !matches!(id.as_str(), "github" | "google") {
                                return;
                            }
                            if account.is_some() {
                                action.run(
                                    "POST",
                                    format!("/api/me/oauth/{id}/start"),
                                    None,
                                    move |v| {
                                        if let Some(url) = v["authorization_url"].as_str() {
                                            go(url);
                                        }
                                    },
                                );
                            } else {
                                action.busy.revise(|b| *b = true);
                                go(&format!(
                                    "/api/auth/{id}/start?{}",
                                    query("next", safe_return(&login_return()))
                                ));
                            }
                        })
                }),
            ))
            .build(scope)
    })
}

pub fn oauth_complete() -> impl Widget {
    widget(|scope| {
        let action = Resource::new(scope);
        let hash = window().location().hash().unwrap_or_default();
        let code = url::form_urlencoded::parse(hash.trim_start_matches('#').as_bytes())
            .find(|(k, _)| k == "code")
            .map(|(_, v)| v.into_owned());
        if let Ok(history) = window().history() {
            let _ = history.replace_state_with_url(&JsValue::NULL, "", Some("/auth/complete"));
        }
        if let Some(code) = code {
            action.run(
                "POST",
                "/api/auth/complete".into(),
                Some(json!({"code":code})),
                move |v| match save_auth(&v) {
                    Ok(()) => go(safe_return(v["next"].as_str().unwrap_or("/me"))),
                    Err(error) => action.error.revise(|e| *e = error),
                },
            );
        } else {
            action
                .error
                .revise(|e| *e = "登录结果已过期，请重新发起登录。".into());
        }
        view()
            .class("panel auth-card")
            .fill((
                title("正在完成登录"),
                action.status(),
                nav("返回登录", "/login"),
            ))
            .build(scope)
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
        let handle = scope.cage(string(&session.get(), "handle"));
        let phone = scope.cage(string(&session.get(), "phone"));
        let action = Resource::new(scope);
        let notice = scope.cage(String::new());
        let account = Resource::new(scope);
        account.load("/api/me/account-status");
        let summary = Resource::new(scope);
        summary.load("/api/learn/summary");

        let achievements = Resource::new(scope);
        achievements.load("/api/achievements/my");

        let ranks = Resource::new(scope);
        ranks.load("/api/achievements/profile");
        let initial = match window().location().hash().unwrap_or_default().as_str() {
            "#settings" => "settings",
            "#achievements" => "achievements",
            _ => "profile",
        };
        let selected = scope.cage(initial.to_string());
        let listener = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
            let next = match window().location().hash().unwrap_or_default().as_str() {
                "#settings" => "settings",
                "#achievements" => "achievements",
                _ => "profile",
            };
            selected.revise(|v| *v = next.into());
        });
        if window()
            .add_event_listener_with_callback("hashchange", listener.as_ref().unchecked_ref())
            .is_ok()
        {
            let _ = scope.on_cleanup(move || {
                let _ = window().remove_event_listener_with_callback(
                    "hashchange",
                    listener.as_ref().unchecked_ref(),
                );
                drop(listener);
            });
        }
        view().class("profile-page").fill((
            view().class("profile-hero").fill((
                view().class("profile-cover"),
                view().class("profile-identity").fill((
                    media::avatar(session),
                    view().fill((
                        title("我的学习"),
                        text().class("profile-name").content(session.map(|s|string(s,"name"))),
                        text().class("muted").content(session.map(|s|format!("@{} · {}",string(s,"handle"),string(s,"email")))),
                    )),
                )),
            )),
            sapid::tabs("profile-tabs").theme(theme::orange()).value(selected).on_value_change(move|v|{selected.revise(|s|*s=v.clone());let _=window().history().and_then(|h|h.replace_state_with_url(&JsValue::NULL,"",Some(&format!("#{}",v))));}).grow(true)
            .tab(sapid::tab("profile").label("个人资料").content(move || view().class("cards profile-columns").fill((
            view().class("panel account-information").fill((heading("账户信息"),p(format!("邮箱地址\n{}",string(&session.get(),"email"))),text().kind(TextKind::Paragraph).content(session.map(|s|format!("用户标识\n@{}",string(s,"handle")))),text().kind(TextKind::Paragraph).content(session.map(|s|format!("手机号码\n{}",if string(s,"phone").is_empty(){"未设置".into()}else{string(s,"phone")}))),p(format!("注册时间\n{}",if string(&session.get(),"created_at").is_empty(){"—".into()}else{string(&session.get(),"created_at")})))),
            view().class("panel").fill((heading("编辑资料"),field_state("Handle（用户标识）",handle,Purpose::Text,false.into(),action.busy.map(|b|!*b).into()),text().kind(TextKind::Paragraph).class("muted").content("唯一标识，可随时修改；3–32 位，以英文字母开头，可含数字、下划线和短横线。"),field_state("昵称",name,Purpose::Text,false.into(),action.busy.map(|b|!*b).into()),field_state("手机（可选）",phone,Purpose::Tel,false.into(),action.busy.map(|b|!*b).into()),
                action_button().label("保存资料").enabled(action.busy.map(|b|!*b)).on_press(move||action.run("PUT","/api/me".into(),Some(json!({"name":name.get().trim(),"phone":phone.get().trim(),"handle":handle.get().trim()})),move|value|{let admin=session.get()["is_admin"].clone();let mut user=value;user["is_admin"]=admin;handle.revise(|h|*h=string(&user,"handle"));session.revise(|s|*s=user);notice.revise(|s|*s="资料已保存".into());})),
                view().class("actions avatar-actions").fill((action_button().label("上传头像（居中裁剪）").variant(sapid::ButtonVariant::Outline).enabled(action.busy.map(|b|!*b)).on_press(move||media::avatar_upload(action,session)),
                danger_button().class("avatar-delete").variant(sapid::ButtonVariant::Subtle).label("删除头像").enabled(action.busy.map(|b|!*b)).on_press(move||{if confirm("删除当前头像？"){action.run("DELETE","/api/me/avatar".into(),None,move|v|{session.revise(|s|s["avatar"]=v["avatar"].clone());notice.revise(|s|*s="头像已删除".into());});}}),)),action.status(),success_notice(notice,action),
            )),
            account_security(account),
            ))))
            .tab(sapid::tab("achievements").label("成就").content(move || view().class("achievement-page").fill((
                view().class("panel").fill((heading("学习概览"),view().class("stat-grid").fill((dashboard::stat(summary,"学习天数","learning_days","天"),dashboard::stat(summary,"词汇积累","total_vocabulary_count","个"),dashboard::stat(ranks,"经验值","total_xp","XP"),dashboard::stat(ranks,"连续学习","current_streak_days","天"))),summary.status(),ranks.status(),text().class("badge").content(ranks.data.map(|v|format!("{} · 距下一级还需 {} XP",string(&v["rank"],"name_zh"),string(v,"xp_to_next_rank")))))),
                view().class("panel").fill((heading("我的成就"),achievements.status(),empty(achievements,"还没有成就记录，坚持练习解锁第一枚徽章吧。"),view().class("achievement-grid").fill(each(achievements.data.map(items)).key(|v|v.to_string()).seed(|v|view().class(if v["is_completed"]==true{"achievement-card earned"}else{"achievement-card"}).fill((text().class("scene-emoji").content(if string(&v,"icon").is_empty(){"🏅".into()}else{string(&v,"icon")}),heading(string(&v,"name_zh")),p(string(&v,"description_zh")),text().class("badge").content(format!("{} / {} · {} XP",string(&v,"progress"),string(&v,"requirement_value"),string(&v,"xp_reward"))))))))),
            ))))
            .tab(sapid::tab("settings").label("设置").content(providers)),
        )).build(scope)
    })
}
fn verified_login_count(account: &Value) -> usize {
    items(&account["linked_identities"])
        .iter()
        .filter(|i| i["verified"] == true)
        .count()
}

fn account_security(account: Resource) -> impl Widget {
    widget(move |scope| {
        let action = Resource::new(scope);
        let notice = scope.cage(String::new());
        let old = scope.cage(String::new());
        let password = scope.cage(String::new());
        let repeat = scope.cage(String::new());
        let password_open = scope.cage(false);
        let enabled = scope.bond(move || {
            !action.busy.get() && !account.busy.get() && account.data.get().is_object()
        });
        view().class("panel").fill((
            heading("账户安全"),p("管理你的密码与第三方登录方式。"),account.status(),
            sapid::dialog("password-editor").theme(theme::orange()).open(password_open).on_open_change(move|v|password_open.revise(|b|*b=v)).label(account.data.map(|v|if v["has_password"]==true{"修改密码"}else{"设置密码"}.into())).title("账户密码").close_label("关闭密码设置").content(move || view().class("dialog-form").fill((
            when(account.data.map(|v|v["has_password"]==true), move || field_state("当前密码",old,Purpose::Password,false.into(),enabled.into())),
            field_state("新密码",password,Purpose::Password,false.into(),enabled.into()),
            field_state("确认新密码",repeat,Purpose::Password,false.into(),enabled.into()),
            action_button().label(account.data.map(|v|if v["has_password"]==true{"更新密码"}else{"设置密码"}.into())).enabled(enabled).on_press(move || {
                if password.get().len()<8||password.get()!=repeat.get(){action.error.revise(|e|*e="新密码至少 8 字节，两次输入必须一致".into());return;}
                action.run("POST","/api/me/change-password".into(),Some(json!({"current_password":if old.get().is_empty(){Value::Null}else{json!(old.get())},"new_password":password.get()})),move |value|{
                    if let Some(store)=storage() { let _=store.set_item("colang.token", &string(&value,"access_token")); }
                    old.revise(String::clear);password.revise(String::clear);repeat.revise(String::clear);
                    password_open.revise(|v|*v=false);notice.revise(|s|*s="密码已更新".into());account.load("/api/me/account-status");
                });
            }),action.status(),
            ))),
            social_buttons(Some(account),action),
            each(account.data.map(|v|items(&v["linked_identities"]))).key(|v|v.to_string()).seed(move |identity| {
                let id=string(&identity,"id");let verified=identity["verified"]==true;
                let provider=match identity["provider"].as_str(){Some("google")=>"Google",Some("github")=>"GitHub",_=>"第三方账户"};
                view().class("linked-account").fill((
                    p(format!("{provider} {}{}",string(&identity,"email"),if verified{"（已绑定）"}else{"（需重新授权验证）"})),
                    danger_button().label(format!("解除 {provider} 绑定"))
                        .enabled(account.data.map(move |a|!action.busy.get()&&!account.busy.get()&&(a["has_password"]==true||verified_login_count(a)>1||!verified)))
                        .on_press(move || {
                            if confirm(&format!("解除 {provider} 账户的绑定？")) {
                                action.run("DELETE",format!("/api/me/oauth/{id}"),None,move |_|{
                                    notice.revise(|s|*s=format!("已解除 {provider} 绑定"));account.load("/api/me/account-status");
                                });
                            }
                        }),
                ))
            }),
            when(account.data.map(|v|v["has_password"]!=true&&verified_login_count(v)==1),||p("请先设置密码或绑定其他账户，才能解除最后一种登录方式。")),
            action.status(),success_notice(notice,action),
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
    super::providers::panel()
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
