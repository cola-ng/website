
#[handler]
pub async fn evaluate(req: &mut Request, res: &mut Response) -> AppResult<()> {
    // Parse request body
    let body_bytes = req.payload().await.map_err(|e| {
        tracing::error!("evaluate_pronunciation: read_body error: {:?}", e);
        StatusError::bad_request().brief("failed to read request body")
    })?;

    let input: EvaluateRequest = serde_json::from_slice(&body_bytes).map_err(|e| {
        tracing::error!("evaluate_pronunciation: parse_json error: {:?}", e);
        StatusError::bad_request().brief("invalid json")
    })?;

    // Decode audio
    let audio_data = BASE64
        .decode(&input.audio_base64)
        .map_err(|_| StatusError::bad_request().brief("invalid base64 audio"))?;

    if audio_data.is_empty() {
        return Err(StatusError::bad_request()
            .brief("audio data is empty")
            .into());
    }

    // Get AI provider
    let provider = create_provider_from_env()
        .ok_or_else(|| StatusError::internal_server_error().brief("AI provider not configured"))?;

    // Get ASR service
    let asr = provider
        .asr()
        .ok_or_else(|| StatusError::internal_server_error().brief("ASR service not available"))?;

    // Transcribe audio
    tracing::info!("Evaluating pronunciation using {} ASR...", provider.name());
    let asr_result: AsrResponse =
        asr.transcribe(audio_data, Some("en"))
            .await
            .map_err(|e: AiProviderError| {
                tracing::error!("{} ASR error: {:?}", provider.name(), e);
                StatusError::internal_server_error().brief(e.to_string())
            })?;

    let transcribed_text = asr_result.text.trim().to_string();
    tracing::info!("ASR result: {}", transcribed_text);

    if transcribed_text.is_empty() {
        return Err(StatusError::bad_request()
            .brief("Could not transcribe audio - no speech detected")
            .into());
    }

    // Calculate scores by comparing transcribed text with reference
    let (overall_score, pronunciation_score, fluency_score, intonation_score, feedback) =
        calculate_pronunciation_score(&transcribed_text, &input.reference_text);

    res.render(Json(EvaluateResponse {
        transcribed_text,
        overall_score,
        pronunciation_score,
        fluency_score,
        intonation_score,
        feedback,
    }));
    Ok(())
}
