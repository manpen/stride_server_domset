INSERT INTO InstanceData (did, hash, data) VALUES 
    (1, 'dummyhash', 'p ds 2 1\n1 2\n'),
    (2, 'dummyhash2', 'p ds 3 2\n1 2\n 2 3\n');

INSERT INTO Instance (iid, data_did, nodes, edges, name, description, submitted_by) VALUES
    (1, 1, 10, 1, 'Dummy Instance', 'This is a dummy instance for testing.', 'tester'),
    (2, 2, 3, 2, 'Dummy Instance 2', 'This is a dummy instance for testing. 2', 'tester ');