-- Default content so a fresh install is immediately usable. Ids are fixed here
-- (SQLite has no UUID generator); they only need to be stable and distinct.

INSERT INTO forum.ranks (id, title, min_posts, is_special, badge) VALUES
    (X'00000000000040008000000000000001', 'Newcomer',      0,    0, NULL),
    (X'00000000000040008000000000000002', 'Member',        10,   0, NULL),
    (X'00000000000040008000000000000003', 'Regular',       50,   0, NULL),
    (X'00000000000040008000000000000004', 'Senior Member', 200,  0, NULL),
    (X'00000000000040008000000000000005', 'Veteran',       1000, 0, NULL),
    (X'00000000000040008000000000000006', 'Administrator', 0,    1, 'admin');

INSERT INTO forum.categories (id, name, description, position) VALUES
    (X'00000000000040008000000000000010', 'General', 'General discussion', 0);

INSERT INTO forum.forums (id, category_id, name, description, position) VALUES
    (X'00000000000040008000000000000011',
     X'00000000000040008000000000000010',
     'Welcome', 'Introduce yourself and start a conversation', 0);

INSERT INTO forum.report_reasons (id, title, description, position) VALUES
    (X'00000000000040008000000000000020', 'Spam',                'Publicité, lien non sollicité ou contenu répétitif.', 0),
    (X'00000000000040008000000000000021', 'Contenu inapproprié', 'Contenu choquant, illégal ou contraire aux règles.',  1),
    (X'00000000000040008000000000000022', 'Hors-sujet',          'Ne concerne pas le sujet ou le forum.',               2),
    (X'00000000000040008000000000000023', 'Harcèlement',         'Attaque personnelle, intimidation ou insulte.',       3);
