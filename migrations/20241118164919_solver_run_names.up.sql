ALTER TABLE SolverRun 
    ADD COLUMN name VARCHAR(255),
    ADD COLUMN description TEXT,
    ADD COLUMN user_key VARCHAR(16);

ALTER TABLE SolverRun
    ADD CONSTRAINT unique_solver_user_key UNIQUE (solver_uuid, user_key);