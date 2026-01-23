// @generated automatically by Diesel CLI.

diesel::table! {
    archive_achievement_definitions (id) {
        id -> Int8,
        code -> Text,
        name_en -> Text,
        name_zh -> Text,
        description_en -> Nullable<Text>,
        description_zh -> Nullable<Text>,
        icon -> Nullable<Text>,
        category -> Text,
        rarity -> Text,
        xp_reward -> Int4,
        requirement_type -> Text,
        requirement_value -> Int4,
        requirement_field -> Nullable<Text>,
        is_hidden -> Nullable<Bool>,
        is_active -> Nullable<Bool>,
        sort_order -> Nullable<Int4>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    archive_rank_definitions (id) {
        id -> Int8,
        code -> Text,
        name_en -> Text,
        name_zh -> Text,
        description_en -> Nullable<Text>,
        description_zh -> Nullable<Text>,
        icon -> Nullable<Text>,
        color -> Nullable<Text>,
        min_xp -> Int4,
        level -> Int4,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    archive_user_achievements (id) {
        id -> Int8,
        user_id -> Int8,
        achievement_id -> Int8,
        progress -> Int4,
        is_completed -> Bool,
        completed_at -> Nullable<Timestamptz>,
        notified_at -> Nullable<Timestamptz>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    archive_user_profiles (id) {
        id -> Int8,
        user_id -> Int8,
        total_xp -> Int4,
        current_rank_id -> Nullable<Int8>,
        current_streak_days -> Int4,
        longest_streak_days -> Int4,
        last_activity_date -> Nullable<Date>,
        total_study_minutes -> Int4,
        total_words_mastered -> Int4,
        total_stages -> Int4,
        total_sessions -> Int4,
        joined_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    archive_user_xp_history (id) {
        id -> Int8,
        user_id -> Int8,
        xp_amount -> Int4,
        source_type -> Text,
        source_id -> Nullable<Int8>,
        description -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    asset_context_categories (id) {
        id -> Int8,
        context_id -> Int8,
        category_id -> Int8,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    asset_contexts (id) {
        id -> Int8,
        name_en -> Text,
        name_zh -> Text,
        description_en -> Nullable<Text>,
        description_zh -> Nullable<Text>,
        icon_emoji -> Nullable<Text>,
        display_order -> Nullable<Int4>,
        difficulty -> Nullable<Int2>,
        user_id -> Nullable<Int8>,
        prompt -> Nullable<Text>,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    asset_read_sentences (id) {
        id -> Int8,
        subject_id -> Int8,
        sentence_order -> Int4,
        content_en -> Text,
        content_zh -> Text,
        phonetic_transcription -> Nullable<Text>,
        native_audio_path -> Nullable<Text>,
        difficulty -> Nullable<Int2>,
        focus_sounds -> Nullable<Jsonb>,
        common_mistakes -> Nullable<Jsonb>,
    }
}

diesel::table! {
    asset_read_subjects (id) {
        id -> Int8,
        title_en -> Text,
        title_zh -> Text,
        description_en -> Nullable<Text>,
        description_zh -> Nullable<Text>,
        difficulty -> Nullable<Int2>,
        subject_type -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    asset_script_turns (id) {
        id -> Int8,
        script_id -> Int8,
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
    asset_scripts (id) {
        id -> Int8,
        stage_id -> Int8,
        title_en -> Text,
        title_zh -> Text,
        description_en -> Nullable<Text>,
        description_zh -> Nullable<Text>,
        total_turns -> Nullable<Int4>,
        estimated_duration_seconds -> Nullable<Int4>,
        difficulty -> Nullable<Int2>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    asset_stages (id) {
        id -> Int8,
        name_en -> Text,
        name_zh -> Text,
        description_en -> Nullable<Text>,
        description_zh -> Nullable<Text>,
        icon_emoji -> Nullable<Text>,
        display_order -> Nullable<Int4>,
        difficulty -> Nullable<Int2>,
        is_active -> Nullable<Bool>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    asset_word_sentences (id) {
        id -> Int8,
        word_id -> Int8,
        sentence_id -> Int8,
        sentence_order -> Int4,
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
    dict_definitions (id) {
        id -> Int8,
        word_id -> Int8,
        language -> Text,
        definition -> Text,
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
    dict_dictionaries (id) {
        id -> Int8,
        name_en -> Text,
        name_zh -> Text,
        short_en -> Text,
        short_zh -> Text,
        description_en -> Nullable<Text>,
        description_zh -> Nullable<Text>,
        version -> Nullable<Text>,
        publisher -> Nullable<Text>,
        license_type -> Nullable<Text>,
        license_url -> Nullable<Text>,
        source_url -> Nullable<Text>,
        total_entries -> Nullable<Int8>,
        is_active -> Nullable<Bool>,
        is_official -> Nullable<Bool>,
        priority_order -> Nullable<Int4>,
        created_by -> Nullable<Int8>,
        updated_by -> Nullable<Int8>,
        updated_at -> Timestamptz,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_etymologies (id) {
        id -> Int8,
        origin_language -> Nullable<Text>,
        origin_word -> Nullable<Text>,
        origin_meaning -> Nullable<Text>,
        language -> Text,
        etymology -> Text,
        first_attested_year -> Nullable<Int4>,
        historical_forms -> Nullable<Jsonb>,
        cognate_words -> Nullable<Jsonb>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_forms (id) {
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
    dict_frequencies (id) {
        id -> Int8,
        word_id -> Int8,
        corpus_name -> Text,
        corpus_type -> Nullable<Text>,
        band -> Nullable<Text>,
        rank -> Nullable<Int4>,
        frequency_per_million -> Nullable<Int4>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_images (id) {
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
    dict_import_batches (id) {
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
    dict_parts (id) {
        id -> Int8,
        word_id -> Int8,
        part_id -> Int8,
        range_begin -> Int2,
        range_until -> Nullable<Int2>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_pronunciations (id) {
        id -> Int8,
        word_id -> Int8,
        definition_id -> Nullable<Int8>,
        ipa -> Text,
        audio_url -> Nullable<Text>,
        audio_path -> Nullable<Text>,
        dialect -> Nullable<Text>,
        gender -> Nullable<Text>,
        is_primary -> Nullable<Bool>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_relations (id) {
        id -> Int8,
        word_id -> Int8,
        relation_type -> Nullable<Text>,
        related_word_id -> Int8,
        semantic_field -> Nullable<Text>,
        relation_strength -> Nullable<Int2>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_searched_words (id) {
        id -> Int8,
        user_id -> Int8,
        word_id -> Nullable<Int8>,
        #[max_length = 255]
        word -> Varchar,
        searched_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    dict_sentences (id) {
        id -> Int8,
        language -> Text,
        sentence -> Text,
        source -> Nullable<Text>,
        author -> Nullable<Text>,
        difficulty -> Nullable<Int4>,
        is_common -> Nullable<Bool>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_translations (id) {
        id -> Int8,
        origin_entity -> Text,
        origin_id -> Int8,
        language -> Text,
        translation -> Text,
        context -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_word_categories (id) {
        id -> Int8,
        word_id -> Int8,
        category_id -> Int8,
        confidence -> Int2,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_word_dictionaries (id) {
        id -> Int8,
        word_id -> Int8,
        dictionary_id -> Int8,
        definition_id -> Nullable<Int8>,
        priority_order -> Nullable<Int4>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_word_etymologies (id) {
        id -> Int8,
        word_id -> Int8,
        etymology_id -> Int8,
        priority_order -> Nullable<Int4>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_word_sentences (id) {
        id -> Int8,
        word_id -> Int8,
        definition_id -> Nullable<Int8>,
        sentence_id -> Int8,
        priority_order -> Nullable<Int4>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    dict_words (id) {
        id -> Int8,
        word -> Text,
        word_lower -> Text,
        word_type -> Nullable<Text>,
        language -> Nullable<Text>,
        frequency -> Nullable<Int2>,
        difficulty -> Nullable<Int2>,
        syllable_count -> Nullable<Int2>,
        is_lemma -> Nullable<Bool>,
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
    learn_chat_issues (id) {
        id -> Int8,
        user_id -> Int8,
        chat_id -> Int8,
        chat_turn_id -> Int8,
        issue_type -> Text,
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
    learn_chat_turns (id) {
        id -> Int8,
        user_id -> Int8,
        chat_id -> Int8,
        speaker -> Text,
        use_lang -> Text,
        content_en -> Text,
        content_zh -> Text,
        audio_path -> Nullable<Text>,
        duration_ms -> Nullable<Int4>,
        words_per_minute -> Nullable<Float4>,
        pause_count -> Nullable<Int4>,
        hesitation_count -> Nullable<Int4>,
        status -> Text,
        error -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    learn_chats (id) {
        id -> Int8,
        user_id -> Int8,
        title -> Text,
        context_id -> Nullable<Int8>,
        duration_ms -> Nullable<Int4>,
        pause_count -> Nullable<Int4>,
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
        difficulty -> Nullable<Int4>,
        context -> Nullable<Text>,
        audio_timestamp -> Nullable<Int4>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    learn_practices (id) {
        id -> Int8,
        user_id -> Int8,
        success_level -> Nullable<Int4>,
        notes -> Nullable<Text>,
        updated_at -> Timestamptz,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    learn_read_practices (id) {
        id -> Int8,
        user_id -> Int8,
        sentence_id -> Nullable<Int8>,
        practice_id -> Text,
        user_audio_path -> Nullable<Text>,
        pronunciation_score -> Nullable<Int4>,
        fluency_score -> Nullable<Int4>,
        intonation_score -> Nullable<Int4>,
        overall_score -> Nullable<Int4>,
        detected_errors -> Nullable<Jsonb>,
        ai_feedback_en -> Nullable<Text>,
        ai_feedback_zh -> Nullable<Text>,
        waveform_data -> Nullable<Jsonb>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    learn_read_progress (id) {
        id -> Int8,
        user_id -> Int8,
        exercise_id -> Int8,
        current_sentence_order -> Nullable<Int4>,
        progress_percent -> Nullable<Int4>,
        completed_at -> Nullable<Timestamptz>,
        last_practiced_at -> Nullable<Timestamptz>,
        practice_count -> Nullable<Int4>,
        average_score -> Nullable<Int4>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
    }
}

diesel::table! {
    learn_script_progress (id) {
        id -> Int8,
        user_id -> Int8,
        stage_id -> Int8,
        script_id -> Int8,
        progress_percent -> Nullable<Int4>,
        completed_at -> Nullable<Timestamptz>,
        last_practiced_at -> Nullable<Timestamptz>,
        practice_count -> Nullable<Int4>,
        best_score -> Nullable<Int4>,
        created_at -> Timestamptz,
        updated_at -> Timestamptz,
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
    learn_write_practices (id) {
        id -> Int8,
        user_id -> Int8,
        word_id -> Int8,
        practice_id -> Text,
        success_level -> Nullable<Int4>,
        notes -> Nullable<Text>,
        updated_at -> Timestamptz,
        created_at -> Timestamptz,
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
    taxon_categories (id) {
        id -> Int8,
        name_en -> Text,
        name_zh -> Text,
        domain_id -> Int8,
        parent_id -> Nullable<Int8>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    taxon_domains (id) {
        id -> Int8,
        name_en -> Text,
        name_zh -> Text,
        created_at -> Timestamptz,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    archive_achievement_definitions,
    archive_rank_definitions,
    archive_user_achievements,
    archive_user_profiles,
    archive_user_xp_history,
    asset_context_categories,
    asset_contexts,
    asset_read_sentences,
    asset_read_subjects,
    asset_script_turns,
    asset_scripts,
    asset_stages,
    asset_word_sentences,
    auth_codes,
    base_passwords,
    base_role_permissions,
    base_role_users,
    base_roles,
    base_users,
    dict_definitions,
    dict_dictionaries,
    dict_etymologies,
    dict_forms,
    dict_frequencies,
    dict_images,
    dict_import_batches,
    dict_parts,
    dict_pronunciations,
    dict_relations,
    dict_searched_words,
    dict_sentences,
    dict_translations,
    dict_word_categories,
    dict_word_dictionaries,
    dict_word_etymologies,
    dict_word_sentences,
    dict_words,
    learn_achievements,
    learn_chat_issues,
    learn_chat_turns,
    learn_chats,
    learn_daily_stats,
    learn_issue_words,
    learn_practices,
    learn_read_practices,
    learn_read_progress,
    learn_script_progress,
    learn_suggestions,
    learn_vocabularies,
    learn_write_practices,
    oauth_identities,
    oauth_login_sessions,
    taxon_categories,
    taxon_domains,
);
