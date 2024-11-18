-- Add up migration script here
ALTER TABLE Instance ADD COLUMN best_score INT UNSIGNED;

UPDATE Instance i
SET i.best_score = (
    SELECT MIN(s.score)
    FROM Solution s
    WHERE s.instance_iid = i.iid
);