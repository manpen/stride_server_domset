INSERT INTO InstanceData (hash, data) VALUES ('dummyhash', 'deadbeef');
INSERT INTO InstanceData (hash, data) VALUES ('dummyhash2', 'deadc0de');

INSERT INTO Instance (iid, data_hash, nodes, edges, name, description, submitted_by) 
    VALUES (1, 'dummyhash', 10, 20, 'Dummy Instance', 'This is a dummy instance for testing.', 'tester');

INSERT INTO Instance (iid, data_hash, nodes, edges, name, description, submitted_by) 
    VALUES (2, 'dummyhash2', 2, 1, 'Dummy Instance 2', 'This is a dummy instance for testing. 2', 'tester ');