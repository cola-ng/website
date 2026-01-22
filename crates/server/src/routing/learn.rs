use salvo::prelude::*;

use crate::hoops;

mod achievement;
mod chat;
mod daily_stat;
mod issue_word;
mod practice;
mod reset;
mod suggestion;
mod summary;
mod vocabulary;

pub fn router() -> Router {
    Router::with_path("learn")
        .hoop(hoops::require_auth)
        .push(Router::with_path("summary").get(summary::get_learn_summary))
        .push(
            Router::with_path("issue-words")
                .get(issue_word::list_issue_words)
                .post(issue_word::create_issue_word)
                .delete(reset::reset_issue_words),
        )
        .push(
            Router::with_path("chats")
                .get(chat::list_chats)
                .post(chat::create_chat)
                .delete(reset::reset_chats)
                .push(Router::with_path("{id}").put(chat::update_chat)),
        )
        .push(
            Router::with_path("chat-turns")
                .get(chat::list_chat_turns)
                .post(chat::create_chat_turn)
                .delete(reset::reset_chat_turns),
        )
        .push(
            Router::with_path("chat-annotations")
                .get(chat::list_chat_annotations)
                .post(chat::create_chat_annotation)
                .delete(reset::reset_chat_annotations),
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
                .delete(reset::reset_vocabulary),
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
        .push(Router::with_path("reset-all").delete(reset::reset_all_learn_data))
}
