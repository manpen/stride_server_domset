-- Add down migration script here
ALTER TABLE SolverRun
    DROP COLUMN num_scheduled;

ALTER TABLE Instance
    DROP COLUMN min_deg,
    DROP COLUMN max_deg,
    DROP COLUMN num_ccs,
    DROP COLUMN nodes_largest_cc,
    DROP COLUMN planar,
    DROP COLUMN bipartite,
    DROP COLUMN diameter,
    DROP COLUMN tree_width;