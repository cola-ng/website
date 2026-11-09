use salvo::prelude::*;

use crate::hoops::{self, require_subscribed};

mod achievement;
mod chat;
mod daily;
mod daily_stat;
mod favorite;
mod heartbeat;
mod issue_word;
mod practice;
mod pronunciation;
mod reset;
mod review;
mod suggestion;
mod summary;
mod vocabulary;

pub fn router() -> Router {
    Router::with_path("learn")
        .hoop(hoops::require_auth)
        .push(Router::with_path("summary").get(summary::get_learn_summary))
        .push(Router::with_path("audios/{user_id}/{filename}").get(chat::serve_audio))
        .push(
            Router::with_path("issue-words")
                .get(issue_word::list_issue_words)
                .post(issue_word::create_issue_word)
                .delete(reset::reset_issue_words),
        )
        .push(
            Router::with_path("pronunciation/evaluate")
                .hoop(require_subscribed)
                .post(pronunciation::evaluate),
        )
        .push(
            Router::with_path("chats")
                .get(chat::list_chats)
                .post(chat::create_chat)
                .delete(reset::reset_chats)
                .push(
                    Router::with_path("{id}")
                        .put(chat::update_chat)
                        .push(
                            Router::with_path("send")
                                .post(chat::send_chat)
                                .push(Router::with_path("stream").post(chat::send_stream)),
                        )
                        .push(Router::with_path("reset").post(chat::reset_chat))
                        .push(Router::with_path("issues").get(chat::list_chat_issues))
                        .push(Router::with_path("turns").get(chat::list_turns)),
                )
                .push(Router::with_path("turns").get(chat::list_turns))
                .push(
                    Router::with_path("turns/{id}")
                        .get(chat::get_turn)
                        .delete(chat::delete_turn)
                        .push(Router::with_path("issues").get(chat::list_turn_issues))
                        .push(Router::with_path("audio").get(chat::serve_turn_audio)),
                ),
        )
        .push(
            Router::with_path("write-practices")
                .get(practice::list_write_practices)
                .post(practice::create_write_practice)
                .delete(reset::reset_write_practices),
        )
        .push(
            Router::with_path("read-practices")
                .get(practice::list_read_practices)
                .post(practice::create_read_practice)
                .delete(reset::reset_read_practices),
        )
        .push(
            Router::with_path("vocabulary")
                .get(vocabulary::list_vocabulary)
                .post(vocabulary::create_vocabulary)
                .delete(reset::reset_vocabulary)
                .push(Router::with_path("toggle").post(vocabulary::toggle_vocabulary))
                .push(Router::with_path("check").post(vocabulary::check_vocabulary)),
        )
        .push(
            Router::with_path("daily-stats")
                .get(daily_stat::list_daily_stats)
                .post(daily_stat::upsert_daily_stat)
                .delete(reset::reset_daily_stats),
        )
        .push(
            Router::with_path("achievements")
                .get(achievement::list_achievements)
                .delete(reset::reset_achievements),
        )
        .push(
            Router::with_path("suggestions")
                .get(suggestion::list_suggestions)
                .post(suggestion::create_suggestion)
                .delete(reset::reset_suggestions),
        )
        .push(
            Router::with_path("review")
                .push(Router::with_path("items").get(review::get_review_items))
                .push(Router::with_path("stats").get(review::get_review_stats))
                .push(Router::with_path("submit").post(review::submit_review)),
        )
        .push(
            Router::with_path("favorites")
                .get(favorite::list_favorites)
                .post(favorite::create_favorite)
                .delete(favorite::delete_favorite)
                .push(Router::with_path("toggle").post(favorite::toggle_favorite))
                .push(Router::with_path("check").post(favorite::check_favorites)),
        )
        .push(
            Router::with_path("daily")
                .get(daily::get_daily_data)
                .push(Router::with_path("tasks/{id}").put(daily::update_task)),
        )
        .push(Router::with_path("heartbeat").post(heartbeat::heartbeat))
        .push(Router::with_path("study-time").get(heartbeat::get_study_time_stats))
}
