-- Remove seed data for review page
DELETE FROM learn_vocabularies WHERE user_id = 1;
DELETE FROM learn_issue_words WHERE user_id = 1;
DELETE FROM learn_daily_stats WHERE user_id = 1;
