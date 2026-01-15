// @generated automatically by Diesel CLI.

diesel::table! {
    assistant_conversations (id) {
        id -> Int8,
        user_id -> Int8,
        session_id -> Text,
        external_app -> Nullable<Text>,
        message_count -> Nullable<Int4>,
        ai_suggestions_count -> Nullable<Int4>,
        grammar_corrections_count -> Nullable<Int4>,
        translations_count -> Nullable<Int4>,
        started_at -> Timestamptz,
        ended_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    assistant_suggestions (id) {
        id -> Int8,
        user_id -> Int8,
        conversation_id -> Int8,
        suggestion_type -> Nullable<Text>,
        suggested_text -> Text,
        was_accepted -> Nullable<Bool>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    classic_dialogue_clips (id) {
        id -> Int8,
        source_id -> Int8,
        clip_title_en -> Text,
        clip_title_zh -> Text,
        start_time_seconds -> Nullable<Int4>,
        end_time_seconds -> Nullable<Int4>,
        video_url -> Nullable<Text>,
        transcript_en -> Text,
        transcript_zh -> Text,
        key_vocabulary -> Nullable<Jsonb>,
        cultural_notes -> Nullable<Text>,
        grammar_points -> Nullable<Jsonb>,
        difficulty_vocab -> Nullable<Int4>,
        difficulty_speed -> Nullable<Int4>,
        difficulty_slang -> Nullable<Int4>,
        popularity_score -> Nullable<Int4>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    classic_dialogue_sources (id) {
        id -> Int8,
        source_type -> Text,
        title -> Text,
        year -> Nullable<Int4>,
        description_en -> Nullable<Text>,
        description_zh -> Nullable<Text>,
        thumbnail_url -> Nullable<Text>,
        imdb_id -> Nullable<Text>,
        difficulty_level -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    conversation_annotations (id) {
        id -> Int8,
        user_id -> Int8,
        conversation_id -> Int8,
        annotation_type -> Text,
        start_position -> Nullable<Int4>,
        end_position -> Nullable<Int4>,
        original_text -> Nullable<Text>,
        suggested_text -> Nullable<Text>,
        description_en -> Nullable<Text>,
        description_zh -> Nullable<Text>,
        severity -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    conversations (id) {
        id -> Int8,
        user_id -> Int8,
        session_id -> Text,
        speaker -> Text,
        use_lang -> Text,
        content_en -> Text,
        content_zh -> Text,
        audio_path -> Nullable<Text>,
        duration_ms -> Nullable<Int4>,
        words_per_minute -> Nullable<Float4>,
        pause_count -> Nullable<Int4>,
        hesitation_count -> Nullable<Int4>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    daily_stats (id) {
        id -> Int8,
        user_id -> Int8,
        stat_date -> Date,
        minutes_studied -> Nullable<Int4>,
        words_practiced -> Nullable<Int4>,
        sessions_completed -> Nullable<Int4>,
        errors_corrected -> Nullable<Int4>,
        new_words_learned -> Nullable<Int4>,
        review_words_count -> Nullable<Int4>,
    }
}

diesel::table! {
    desktop_auth_codes (id) {
        id -> Int8,
        user_id -> Int8,
        code_hash -> Text,
        redirect_uri -> Text,
        state -> Text,
        expires_at -> Timestamptz,
        used_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dialogue_turns (id) {
        id -> Int8,
        scene_dialogue_id -> Int8,
        turn_number -> Int4,
        speaker_role -> Text,
        speaker_name -> Nullable<Text>,
        content_en -> Text,
        content_zh -> Text,
        audio_path -> Nullable<Text>,
        phonetic_transcription -> Nullable<Text>,
        key_phrases -> Nullable<Jsonb>,
        notes -> Nullable<Text>,
    }
}

diesel::table! {
    issue_words (id) {
        id -> Int8,
        user_id -> Int8,
        word -> Text,
        issue_type -> Text,
        description_en -> Nullable<Text>,
        description_zh -> Nullable<Text>,
        last_picked_at -> Nullable<Timestamptz>,
        pick_count -> Int4,
        next_review_at -> Nullable<Timestamptz>,
        review_interval_days -> Nullable<Int4>,
        difficulty_level -> Nullable<Int4>,
        context -> Nullable<Text>,
        audio_timestamp -> Nullable<Int4>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    key_phrases (id) {
        id -> Int8,
        phrase_en -> Text,
        phrase_zh -> Text,
        phonetic_transcription -> Nullable<Text>,
        usage_context -> Nullable<Text>,
        example_sentence_en -> Nullable<Text>,
        example_sentence_zh -> Nullable<Text>,
        category -> Nullable<Text>,
        formality_level -> Nullable<Text>,
        frequency_score -> Nullable<Int4>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    learning_records (id) {
        id -> Int8,
        user_id -> Int8,
        record_type -> Text,
        content -> Jsonb,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    learning_sessions (id) {
        id -> Int8,
        session_id -> Text,
        user_id -> Int8,
        session_type -> Nullable<Text>,
        scenario_id -> Nullable<Int8>,
        scene_dialogue_id -> Nullable<Int8>,
        classic_clip_id -> Nullable<Int8>,
        started_at -> Timestamptz,
        ended_at -> Nullable<Timestamptz>,
        duration_seconds -> Nullable<Int4>,
        total_words_spoken -> Nullable<Int4>,
        average_wpm -> Nullable<Float4>,
        error_count -> Nullable<Int4>,
        correction_count -> Nullable<Int4>,
        notes -> Nullable<Text>,
        ai_summary_en -> Nullable<Text>,
        ai_summary_zh -> Nullable<Text>,
    }
}

diesel::table! {
    oauth_identities (id) {
        id -> Int8,
        provider -> Text,
        provider_user_id -> Text,
        email -> Nullable<Text>,
        user_id -> Nullable<Int8>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    oauth_login_sessions (id) {
        id -> Int8,
        provider -> Text,
        state -> Text,
        redirect_uri -> Text,
        created_at -> Timestamptz,
        expires_at -> Timestamptz,
    }
}

diesel::table! {
    reading_exercises (id) {
        id -> Int8,
        title_en -> Text,
        title_zh -> Text,
        description_en -> Nullable<Text>,
        description_zh -> Nullable<Text>,
        difficulty_level -> Nullable<Text>,
        exercise_type -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    reading_practice_attempts (id) {
        id -> Int8,
        user_id -> Int8,
        sentence_id -> Int8,
        session_id -> Text,
        user_audio_path -> Nullable<Text>,
        pronunciation_score -> Nullable<Int4>,
        fluency_score -> Nullable<Int4>,
        intonation_score -> Nullable<Int4>,
        overall_score -> Nullable<Int4>,
        detected_errors -> Nullable<Jsonb>,
        ai_feedback_en -> Nullable<Text>,
        ai_feedback_zh -> Nullable<Text>,
        waveform_data -> Nullable<Jsonb>,
        attempted_at -> Timestamptz,
    }
}

diesel::table! {
    reading_sentences (id) {
        id -> Int8,
        exercise_id -> Int8,
        sentence_order -> Int4,
        content_en -> Text,
        content_zh -> Text,
        phonetic_transcription -> Nullable<Text>,
        native_audio_path -> Nullable<Text>,
        focus_sounds -> Nullable<Jsonb>,
        common_mistakes -> Nullable<Jsonb>,
    }
}

diesel::table! {
    role_permissions (id) {
        id -> Int8,
        role_id -> Int8,
        operation -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    roles (id) {
        id -> Int8,
        name -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    scenarios (id) {
        id -> Int8,
        name_en -> Text,
        name_zh -> Text,
        description_en -> Nullable<Text>,
        description_zh -> Nullable<Text>,
        icon_emoji -> Nullable<Text>,
        difficulty_level -> Nullable<Text>,
        category -> Nullable<Text>,
        display_order -> Nullable<Int4>,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    scene_dialogues (id) {
        id -> Int8,
        scenario_id -> Int8,
        title_en -> Text,
        title_zh -> Text,
        description_en -> Nullable<Text>,
        description_zh -> Nullable<Text>,
        total_turns -> Nullable<Int4>,
        estimated_duration_seconds -> Nullable<Int4>,
        difficulty_level -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    user_achievements (id) {
        id -> Int8,
        user_id -> Int8,
        achievement_type -> Text,
        achievement_name -> Text,
        description_en -> Nullable<Text>,
        description_zh -> Nullable<Text>,
        metadata -> Nullable<Jsonb>,
        earned_at -> Timestamptz,
    }
}

diesel::table! {
    user_roles (user_id, role_id) {
        user_id -> Int8,
        role_id -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    user_vocabulary (id) {
        id -> Int8,
        user_id -> Int8,
        word -> Text,
        word_zh -> Nullable<Text>,
        mastery_level -> Nullable<Int4>,
        first_seen_at -> Timestamptz,
        last_practiced_at -> Nullable<Timestamptz>,
        practice_count -> Nullable<Int4>,
        correct_count -> Nullable<Int4>,
        next_review_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    users (id) {
        id -> Int8,
        email -> Text,
        password_hash -> Text,
        name -> Nullable<Text>,
        phone -> Nullable<Text>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    word_practice_log (id) {
        id -> Int8,
        user_id -> Int8,
        word_id -> Int8,
        session_id -> Text,
        success_level -> Nullable<Int4>,
        notes -> Nullable<Text>,
        practiced_at -> Timestamptz,
    }
}

diesel::joinable!(assistant_conversations -> users (user_id));
diesel::joinable!(assistant_suggestions -> assistant_conversations (conversation_id));
diesel::joinable!(assistant_suggestions -> users (user_id));
diesel::joinable!(classic_dialogue_clips -> classic_dialogue_sources (source_id));
diesel::joinable!(conversation_annotations -> conversations (conversation_id));
diesel::joinable!(conversation_annotations -> users (user_id));
diesel::joinable!(conversations -> users (user_id));
diesel::joinable!(daily_stats -> users (user_id));
diesel::joinable!(desktop_auth_codes -> users (user_id));
diesel::joinable!(dialogue_turns -> scene_dialogues (scene_dialogue_id));
diesel::joinable!(issue_words -> users (user_id));
diesel::joinable!(learning_records -> users (user_id));
diesel::joinable!(learning_sessions -> classic_dialogue_clips (classic_clip_id));
diesel::joinable!(learning_sessions -> scenarios (scenario_id));
diesel::joinable!(learning_sessions -> scene_dialogues (scene_dialogue_id));
diesel::joinable!(learning_sessions -> users (user_id));
diesel::joinable!(oauth_identities -> users (user_id));
diesel::joinable!(reading_practice_attempts -> reading_sentences (sentence_id));
diesel::joinable!(reading_practice_attempts -> users (user_id));
diesel::joinable!(reading_sentences -> reading_exercises (exercise_id));
diesel::joinable!(role_permissions -> roles (role_id));
diesel::joinable!(scene_dialogues -> scenarios (scenario_id));
diesel::joinable!(user_achievements -> users (user_id));
diesel::joinable!(user_roles -> roles (role_id));
diesel::joinable!(user_roles -> users (user_id));
diesel::joinable!(user_vocabulary -> users (user_id));
diesel::joinable!(word_practice_log -> issue_words (word_id));
diesel::joinable!(word_practice_log -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    assistant_conversations,
    assistant_suggestions,
    classic_dialogue_clips,
    classic_dialogue_sources,
    conversation_annotations,
    conversations,
    daily_stats,
    desktop_auth_codes,
    dialogue_turns,
    issue_words,
    key_phrases,
    learning_records,
    learning_sessions,
    oauth_identities,
    oauth_login_sessions,
    reading_exercises,
    reading_practice_attempts,
    reading_sentences,
    role_permissions,
    roles,
    scenarios,
    scene_dialogues,
    user_achievements,
    user_roles,
    user_vocabulary,
    users,
    word_practice_log,
);
