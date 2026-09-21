-- Default content so a fresh install is immediately usable.

-- Default ranks (phpBB-style ladder based on post count).
INSERT INTO forum.ranks (title, min_posts, is_special, badge) VALUES
    ('Newcomer',      0,    FALSE, NULL),
    ('Member',        10,   FALSE, NULL),
    ('Regular',       50,   FALSE, NULL),
    ('Senior Member', 200,  FALSE, NULL),
    ('Veteran',       1000, FALSE, NULL),
    ('Administrator', 0,    TRUE,  'admin');

-- A default category with a welcome forum.
WITH cat AS (
    INSERT INTO forum.categories (name, description, position)
    VALUES ('General', 'General discussion', 0)
    RETURNING id
)
INSERT INTO forum.forums (category_id, name, description, position)
SELECT id, 'Welcome', 'Introduce yourself and start a conversation', 0 FROM cat;
