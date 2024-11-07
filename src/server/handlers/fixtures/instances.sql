INSERT INTO InstanceData (hash, data) VALUES ('dummyhash', 'p ds 2 1\n1 2\n');
INSERT INTO InstanceData (hash, data) VALUES ('dummyhash2', 'p ds 3 2\n1 2\n 2 3\n');

INSERT INTO Instance (iid, data_hash, nodes, edges, name, description, submitted_by) 
    VALUES (1, 'dummyhash', 2, 1, 'Dummy Instance', 'This is a dummy instance for testing.', 'tester');

INSERT INTO Instance (iid, data_hash, nodes, edges, name, description, submitted_by) 
    VALUES (2, 'dummyhash2', 3, 2, 'Dummy Instance 2', 'This is a dummy instance for testing. 2', 'tester ');