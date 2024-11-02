-- Add up migration script here
CREATE TABLE
    IF NOT EXISTS Instance (
        iid INT AUTO_INCREMENT PRIMARY KEY,

        nodes INT UNSIGNED NOT NULL,
        edges INT UNSIGNED NOT NULL,

        name VARCHAR(255),
        description TEXT,
        submitted_by VARCHAR(255),
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,

        INDEX `idx_nodes` (`nodes`),
        INDEX `idx_edges` (`edges`)
    );

CREATE TABLE
    IF NOT EXISTS InstanceData (
            did INT AUTO_INCREMENT PRIMARY KEY,
            instance_iid INT,
            data BLOB,
            FOREIGN KEY (instance_iid) REFERENCES Instance(iid)
    );

CREATE TABLE
    IF NOT EXISTS Tag (
        tid INT AUTO_INCREMENT PRIMARY KEY,
        description TEXT,
        name VARCHAR(255) NOT NULL
    );

CREATE TABLE
    IF NOT EXISTS InstanceTag (
        instance_iid INT,
        tag_tid INT,
        PRIMARY KEY (instance_iid, tag_tid),
        FOREIGN KEY (instance_iid) REFERENCES Instance(iid),
        FOREIGN KEY (tag_tid) REFERENCES Tag(tid)
    );

CREATE TABLE
    IF NOT EXISTS Solution (
        sid INT AUTO_INCREMENT PRIMARY KEY,
        instance_iid INT,
        score INT UNSIGNED,

        solution_time DOUBLE,
        solver VARCHAR(255),

        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
        FOREIGN KEY (instance_iid) REFERENCES Instance(iid)
    );


CREATE TABLE
    IF NOT EXISTS SolutionData (
            sdid INT AUTO_INCREMENT PRIMARY KEY,
            solution_sid INT,
            data BLOB,
            FOREIGN KEY (solution_sid) REFERENCES Solution(sid)
    );

