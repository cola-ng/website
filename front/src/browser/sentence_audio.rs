use super::*;

/// Loading metadata never invokes TTS. Only an explicit click can spend a personal key.
pub fn player(
    kind: &'static str,
    id: i64,
    fallback: Option<String>,
    label: &'static str,
) -> impl Widget {
    widget(move |scope| {
        let state = Resource::new(scope);
        let path = format!("/api/sentence-audio/{kind}/{id}");
        state.load(path.clone());
        let watched=scope.cage(false);
        let refresh_path=path.clone();
        scope.effect(move||{
            if state.data.get()["status"]=="generating" && !watched.get(){
                watched.revise(|v|*v=true);let refresh_path=refresh_path.clone();
                spawn(async move{
                    for _ in 0..100 {
                        gloo_timers::future::TimeoutFuture::new(2000).await;
                        if !watched.is_alive() {return;}
                        let result=request("GET",&refresh_path,None).await;
                        if !watched.is_alive(){return;}
                        match result {
                            Ok(value)=>{let done=value["status"]!="generating";state.data.revise(|v|*v=value);if done{break;}},
                            Err(_)=>break,
                        }
                    }
                    if watched.is_alive(){watched.revise(|v|*v=false);}
                });
            }
        });
        let action = Resource::new(scope);
        let note = scope.cage(String::new());
        let fallback_url = fallback.clone();
        let has_fallback = fallback.is_some();
        view().class("sentence-audio").fill((
            when(state.data.map(move|v|!string(v,"audio_url").is_empty()||v["can_generate"]==true||(has_fallback&&v.is_null())),move||{
                let fallback=fallback_url.clone();
                let path=path.clone();
                action_button().class("sentence-audio-button").label(action.busy.map(move|b|if *b{"正在生成…".into()}else{format!("🔊 {label}")})).enabled(action.busy.map(move|b|!*b&&state.data.get()["status"]!="generating")).on_press(move||{
                    let url=string(&state.data.get(),"audio_url");
                    if !url.is_empty(){media::play(url,action.error);return;}
                    if state.data.get().is_null() && let Some(url)=fallback.clone(){media::play(url,action.error);return;}
                    if !confirm("使用你自己的 TTS API Key 生成这句话的发音？费用由对应平台收取，音频会作为候选提交给平台。白名单平台可自动通过审核。"){return;}
                    let refresh=path.clone();action.run("POST",format!("/api/sentence-audio/{kind}/{id}/generate"),Some(json!({})),move|v|{
                        let url=string(&v,"audio_url");if !url.is_empty(){media::play(url,action.error);}
                        note.revise(|s|*s=match v["status"].as_str(){Some("approved")=>"发音已生成并作为平台正式音频保存。",Some("superseded")=>"发音已生成；原句已更新或已有正式音频，此文件保留为你的候选。",_=>"发音已生成，可播放，已提交管理员审核。"}.into());
                        state.load(refresh.clone());
                    });
                })
            }),
            when(state.data.map(|v|v["status"]=="pending"),||p("候选发音 · 待审核")),
            when(state.data.map(|v|v["status"]=="generating"),||p("发音正在生成，请稍后刷新。")),
            when(note.map(|s|!s.is_empty()),move||view().role(Role::Status).fill(text().content(note))),action.status(),
        )).build(scope)
    })
}

pub fn admin() -> impl Widget {
    widget(|scope| {
        let queue = Resource::new(scope);
        queue.load("/api/admin/sentence-audio");
        let whitelist = Resource::new(scope);
        whitelist.load("/api/admin/sentence-audio/whitelist");
        let catalog = Resource::new(scope);
        catalog.load("/api/ai-providers/catalog");
        let action = Resource::new(scope);
        let status = scope.cage("pending".to_string());
        let notice = scope.cage(String::new());
        let reload =
            move || queue.load(format!("/api/admin/sentence-audio?status={}", status.get()));
        view().class("audio-admin").fill((title("句子发音审核"),
            view().class("panel").fill((heading("自动通过白名单"),p("仅影响之后生成的候选音频。开启后，该平台的个人 TTS 音频可自动补全没有正式发音的句子；不会覆盖已有正式音频。待审核候选仍需人工处理。"),whitelist.status(),catalog.status(),
                each(catalog.data.map(|v|items(v).into_iter().filter(|p|!items(&p["tts_models"]).is_empty()).collect::<Vec<_>>())).key(|v|string(v,"id")).seed(move|p|{
                    let provider=string(&p,"id");let provider_for_change=provider.clone();
                    checkbox_control().label(string(&p,"label")).checked(whitelist.data.map(move|v|items(v).iter().any(|row|string(row,"provider_type")==provider&&row["enabled"]==true))).enabled(action.busy.map(|b|!*b)).on_checked_change(move|enabled|{action.run("PUT","/api/admin/sentence-audio/whitelist".into(),Some(json!({"provider_type":provider_for_change,"enabled":enabled})),move|_|{whitelist.load("/api/admin/sentence-audio/whitelist");notice.revise(|s|*s="白名单已更新".into());});})
                }),
            )),
            view().class("panel").fill((heading("候选发音"),choices(status,&[("pending","待审核"),("approved","已通过"),("rejected","已拒绝"),("superseded","已过期"),("failed","生成失败")]),action_button().label("刷新列表").on_press(reload),queue.status(),empty(queue,"暂无候选发音。"),
                each(queue.data.map(items)).key(|v|v.to_string()).seed(move|row|{
                    let id=row["id"].as_i64().unwrap_or_default();let pending=row["status"]=="pending";
                    widget(move|inner|{
                        let note=inner.cage(String::new());
                        view().class("panel audio-review-card").fill((heading(string(&row,"sentence")),p(format!("{} #{} · {} · {} · 音色 {}",string(&row,"source_kind"),string(&row,"source_id"),string(&row,"provider_type"),string(&row,"model"),string(&row,"voice"))),p(format!("提交用户 #{} · {}",string(&row,"user_id"),string(&row,"created_at"))),p(string(&row,"review_note")),
                            when(!row["audio_path"].is_null(),move||action_button().label("🔊 试听候选发音").on_press(move||media::play(format!("/api/sentence-audio/candidates/{id}/audio"),action.error))),
                            when(pending,move||view().class("form-fields").fill((field("拒绝原因（可选）",note),view().class("actions").fill((action_button().label("通过审核").enabled(action.busy.map(|b|!*b)).on_press(move||{if confirm("将此候选设为平台正式发音？仅在原句未变且没有正式音频时生效。"){action.run("POST",format!("/api/admin/sentence-audio/{id}/review"),Some(json!({"action":"approve"})),move|v|{reload();notice.revise(|s|*s=if v["status"]=="approved"{"已通过审核"}else{"原句已变化或已有正式音频，候选已标记过期"}.into());});}}),danger_button().label("拒绝候选").enabled(action.busy.map(|b|!*b)).on_press(move||action.run("POST",format!("/api/admin/sentence-audio/{id}/review"),Some(json!({"action":"reject","note":note.get()})),move|_|{reload();notice.revise(|s|*s="已拒绝候选发音".into());}))))))),
                        )).build(inner)
                    })
                }),
                when(queue.data.map(|v|v["next_before"].is_i64()),move||action_button().label("下一页候选").on_press(move||{if let Some(before)=queue.data.get()["next_before"].as_i64(){queue.load(format!("/api/admin/sentence-audio?status={}&before={before}",status.get()));}})),
            )),action.status(),success_notice(notice,action),
            widget(move|inner|{inner.effect(move||{status.get();reload();});view().build(inner)}),
        )).build(scope)
    })
}
