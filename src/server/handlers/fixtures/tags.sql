INSERT INTO Tag (tid, name, description, style) VALUES
    (1, "name1", "desc1", 10),
    (2, "name2", "desc2", 20),
    (3, "name3", "desc3", 30)
;

INSERT INTO InstanceTag (instance_iid, tag_tid) VALUES
    (1, 1),
    (1, 2),
    (2, 1);
