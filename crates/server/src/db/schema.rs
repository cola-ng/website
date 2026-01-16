// @generated automatically by Diesel CLI.

diesel::table! {
    asset_classic_clips (id) {
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
    asset_classic_sources (id) {
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
    asset_dialogue_turns (id) {
        id -> Int8,
        dialogue_id -> Int8,
        turn_number -> Int4,
        speaker_role -> Text,
        speaker_name -> Nullable<Text>,
        content_en -> Text,
        content_zh -> Text,
        audio_path -> Nullable<Text>,
        phonetic_transcription -> Nullable<Text>,
        asset_phrases -> Nullable<Jsonb>,
        notes -> Nullable<Text>,
    }
}

diesel::table! {
    asset_dialogues (id) {
        id -> Int8,
        scene_id -> Int8,
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
    asset_phrases (id) {
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
    asset_read_exercises (id) {
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
    asset_read_sentences (id) {
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
    asset_scenes (id) {
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
    auth_codes (id) {
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
    base_passwords (id) {
        id -> Int8,
        user_id -> Int8,
        hash -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    base_role_permissions (id) {
        id -> Int8,
        role_id -> Int8,
        operation -> Text,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    base_role_users (user_id, role_id) {
        role_id -> Int8,
        user_id -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    base_roles (id) {
        id -> Int8,
        code -> Text,
        name -> Text,
        kind -> Text,
        owner_id -> Int8,
        description -> Nullable<Text>,
        updated_by -> Nullable<Int8>,
        updated_at -> Timestamptz,
        created_by -> Nullable<Int8>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    base_users (id) {
        id -> Int8,
        name -> Text,
        email -> Nullable<Text>,
        phone -> Nullable<Text>,
        avatar -> Nullable<Text>,
        display_name -> Nullable<Text>,
        verified_at -> Nullable<Timestamptz>,
        limited_at -> Nullable<Timestamptz>,
        limited_by -> Nullable<Int8>,
        locked_at -> Nullable<Timestamptz>,
        locked_by -> Nullable<Int8>,
        disabled_at -> Nullable<Timestamptz>,
        disabled_by -> Nullable<Int8>,
        inviter_id -> Nullable<Int8>,
        profile -> Jsonb,
        updated_by -> Nullable<Int8>,
        updated_at -> Timestamptz,
        created_by -> Nullable<Int8>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_frequency_bands (id) {
        id -> Int8,
        word_id -> Int8,
        corpus_name -> Text,
        corpus_type -> Nullable<Text>,
        band -> Nullable<Text>,
        rank -> Nullable<Int4>,
        frequency_per_million -> Nullable<Float4>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_import_batch (id) {
        id -> Int8,
        batch_name -> Text,
        source -> Nullable<Text>,
        source_type -> Nullable<Text>,
        total_words -> Nullable<Int4>,
        successful_words -> Nullable<Int4>,
        failed_words -> Nullable<Int4>,
        status -> Nullable<Text>,
        metadata -> Nullable<Jsonb>,
        started_at -> Nullable<Timestamptz>,
        completed_at -> Nullable<Timestamptz>,
        created_by -> Nullable<Int8>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_phrase_words (id) {
        id -> Int8,
        phrase_id -> Int8,
        word_id -> Int8,
        word_position -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_phrases (id) {
        id -> Int8,
        phrase -> Text,
        phrase_lower -> Text,
        phrase_type -> Nullable<Text>,
        meaning_en -> Text,
        meaning_zh -> Nullable<Text>,
        origin -> Nullable<Text>,
        example_en -> Nullable<Text>,
        example_zh -> Nullable<Text>,
        difficulty_level -> Nullable<Int4>,
        frequency_score -> Nullable<Int4>,
        is_active -> Nullable<Bool>,
        created_by -> Nullable<Int8>,
        updated_by -> Nullable<Int8>,
        updated_at -> Timestamptz,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_related_topics (id) {
        id -> Int8,
        word_id -> Int8,
        topic_name -> Text,
        topic_category -> Nullable<Text>,
        relevance_score -> Nullable<Float4>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_word_antonyms (id) {
        id -> Int8,
        word_id -> Int8,
        antonym_word_id -> Int8,
        antonym_type -> Nullable<Text>,
        context -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_word_categories (id) {
        id -> Int8,
        word_id -> Int8,
        category_type -> Nullable<Text>,
        category_name -> Text,
        category_value -> Text,
        confidence_score -> Nullable<Float4>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_word_collocations (id) {
        id -> Int8,
        word_id -> Int8,
        collocation_type -> Nullable<Text>,
        collocated_word_id -> Nullable<Int8>,
        phrase -> Text,
        phrase_en -> Text,
        phrase_zh -> Nullable<Text>,
        frequency_score -> Nullable<Int4>,
        register -> Nullable<Text>,
        example_en -> Nullable<Text>,
        example_zh -> Nullable<Text>,
        is_common -> Nullable<Bool>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_word_definitions (id) {
        id -> Int8,
        word_id -> Int8,
        definition_en -> Text,
        definition_zh -> Nullable<Text>,
        part_of_speech -> Nullable<Text>,
        definition_order -> Nullable<Int4>,
        register -> Nullable<Text>,
        region -> Nullable<Text>,
        context -> Nullable<Text>,
        usage_notes -> Nullable<Text>,
        is_primary -> Nullable<Bool>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_word_etymology (id) {
        id -> Int8,
        word_id -> Int8,
        origin_language -> Nullable<Text>,
        origin_word -> Nullable<Text>,
        origin_meaning -> Nullable<Text>,
        etymology_en -> Nullable<Text>,
        etymology_zh -> Nullable<Text>,
        first_attested_year -> Nullable<Int4>,
        historical_forms -> Nullable<Jsonb>,
        cognate_words -> Nullable<Jsonb>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_word_examples (id) {
        id -> Int8,
        word_id -> Int8,
        definition_id -> Nullable<Int8>,
        sentence_en -> Text,
        sentence_zh -> Nullable<Text>,
        source -> Nullable<Text>,
        author -> Nullable<Text>,
        example_order -> Nullable<Int4>,
        difficulty_level -> Nullable<Int4>,
        is_common -> Nullable<Bool>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_word_family (id) {
        id -> Int8,
        root_word_id -> Int8,
        related_word_id -> Int8,
        relationship_type -> Nullable<Text>,
        morpheme -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_word_forms (id) {
        id -> Int8,
        word_id -> Int8,
        form_type -> Nullable<Text>,
        form -> Text,
        is_irregular -> Nullable<Bool>,
        notes -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_word_images (id) {
        id -> Int8,
        word_id -> Int8,
        image_url -> Nullable<Text>,
        image_path -> Nullable<Text>,
        image_type -> Nullable<Text>,
        alt_text_en -> Nullable<Text>,
        alt_text_zh -> Nullable<Text>,
        is_primary -> Nullable<Bool>,
        created_by -> Nullable<Int8>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_word_pronunciations (id) {
        id -> Int8,
        word_id -> Int8,
        ipa -> Text,
        audio_url -> Nullable<Text>,
        audio_path -> Nullable<Text>,
        dialect -> Nullable<Text>,
        is_primary -> Nullable<Bool>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_word_synonyms (id) {
        id -> Int8,
        word_id -> Int8,
        synonym_word_id -> Int8,
        similarity_score -> Nullable<Float4>,
        context -> Nullable<Text>,
        nuance_notes -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_word_thesaurus (id) {
        id -> Int8,
        word_id -> Int8,
        entry_type -> Nullable<Text>,
        related_word_id -> Int8,
        semantic_field -> Nullable<Text>,
        relationship_strength -> Nullable<Float4>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_word_usage_notes (id) {
        id -> Int8,
        word_id -> Int8,
        note_type -> Nullable<Text>,
        note_en -> Text,
        note_zh -> Nullable<Text>,
        examples_en -> Nullable<Jsonb>,
        examples_zh -> Nullable<Jsonb>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_words (id) {
        id -> Int8,
        word -> Text,
        word_lower -> Text,
        word_type -> Nullable<Text>,
        frequency_score -> Nullable<Int4>,
        difficulty_level -> Nullable<Int4>,
        syllable_count -> Nullable<Int4>,
        is_lemma -> Nullable<Bool>,
        lemma_id -> Nullable<Int8>,
        audio_url -> Nullable<Text>,
        audio_path -> Nullable<Text>,
        phonetic_transcription -> Nullable<Text>,
        ipa_text -> Nullable<Text>,
        word_count -> Nullable<Int4>,
        is_active -> Nullable<Bool>,
        created_by -> Nullable<Int8>,
        updated_by -> Nullable<Int8>,
        updated_at -> Timestamptz,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    learn_achievements (id) {
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
    learn_conversation_annotations (id) {
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
    learn_conversations (id) {
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
    learn_daily_stats (id) {
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
    learn_issue_words (id) {
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
    learn_read_practices (id) {
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
    learn_sessions (id) {
        id -> Int8,
        session_id -> Text,
        user_id -> Int8,
        session_type -> Nullable<Text>,
        scene_id -> Nullable<Int8>,
        dialogue_id -> Nullable<Int8>,
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
    learn_suggestions (id) {
        id -> Int8,
        user_id -> Int8,
        suggestion_type -> Nullable<Text>,
        suggested_text -> Text,
        was_accepted -> Nullable<Bool>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    learn_vocabularies (id) {
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
    learn_word_practices (id) {
        id -> Int8,
        user_id -> Int8,
        word_id -> Int8,
        session_id -> Text,
        success_level -> Nullable<Int4>,
        notes -> Nullable<Text>,
        practiced_at -> Timestamptz,
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

diesel::allow_tables_to_appear_in_same_query!(
    asset_classic_clips,
    asset_classic_sources,
    asset_dialogue_turns,
    asset_dialogues,
    asset_phrases,
    asset_read_exercises,
    asset_read_sentences,
    asset_scenes,
    auth_codes,
    base_passwords,
    base_role_permissions,
    base_role_users,
    base_roles,
    base_users,
    dict_frequency_bands,
    dict_import_batch,
    dict_phrase_words,
    dict_phrases,
    dict_related_topics,
    dict_word_antonyms,
    dict_word_categories,
    dict_word_collocations,
    dict_word_definitions,
    dict_word_etymology,
    dict_word_examples,
    dict_word_family,
    dict_word_forms,
    dict_word_images,
    dict_word_pronunciations,
    dict_word_synonyms,
    dict_word_thesaurus,
    dict_word_usage_notes,
    dict_words,
    learn_achievements,
    learn_conversation_annotations,
    learn_conversations,
    learn_daily_stats,
    learn_issue_words,
    learn_read_practices,
    learn_sessions,
    learn_suggestions,
    learn_vocabularies,
    learn_word_practices,
    oauth_identities,
    oauth_login_sessions,
);
