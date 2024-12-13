-- Add up migration script here
CREATE FULLTEXT INDEX idx_instance_name_description ON `Instance` (`name`, `description`, `submitted_by`);