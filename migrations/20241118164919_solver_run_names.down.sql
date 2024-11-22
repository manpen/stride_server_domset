ALTER TABLE SolverRun
    DROP CONSTRAINT unique_solver_user_key;
    
ALTER TABLE Instance 
    DROP COLUMN `name`,
    DROP COLUMN `description`,
    DROP COLUMN `user_key`;

