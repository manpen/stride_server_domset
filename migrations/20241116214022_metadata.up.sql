-- Add up migration script here
ALTER TABLE SolverRun
    ADD COLUMN num_scheduled INT UNSIGNED;

ALTER TABLE Instance
    ADD COLUMN min_deg INT UNSIGNED,
    ADD COLUMN max_deg INT UNSIGNED,
    ADD COLUMN num_ccs INT UNSIGNED,
    ADD COLUMN nodes_largest_cc INT UNSIGNED,
    ADD COLUMN planar BOOLEAN,
    ADD COLUMN diameter INT UNSIGNED,
    ADD COLUMN tree_width INT UNSIGNED;


