CREATE TABLE statistics_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    timeframe TEXT NOT NULL CHECK (timeframe IN ('last_week', 'last_month', 'last_six_months', 'last_year', 'all_time'))
) STRICT;
INSERT INTO statistics_settings (id, timeframe) VALUES (1, 'last_year');
