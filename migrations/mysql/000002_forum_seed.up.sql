-- Default content so a fresh install is immediately usable. Ids are fixed here
-- (MySQL has no gen_random_uuid()); they only need to be stable and distinct.

INSERT INTO ranks (id, title, min_posts, is_special, badge) VALUES
    (UNHEX('00000000000040008000000000000001'), 'Newcomer',      0,    FALSE, NULL),
    (UNHEX('00000000000040008000000000000002'), 'Member',        10,   FALSE, NULL),
    (UNHEX('00000000000040008000000000000003'), 'Regular',       50,   FALSE, NULL),
    (UNHEX('00000000000040008000000000000004'), 'Senior Member', 200,  FALSE, NULL),
    (UNHEX('00000000000040008000000000000005'), 'Veteran',       1000, FALSE, NULL),
    (UNHEX('00000000000040008000000000000006'), 'Administrator', 0,    TRUE,  'admin');

INSERT INTO categories (id, name, description, position) VALUES
    (UNHEX('00000000000040008000000000000010'), 'General', 'General discussion', 0);

INSERT INTO forums (id, category_id, name, description, position) VALUES
    (UNHEX('00000000000040008000000000000011'),
     UNHEX('00000000000040008000000000000010'),
     'Welcome', 'Introduce yourself and start a conversation', 0);

INSERT INTO report_reasons (id, title, description, position) VALUES
    (UNHEX('00000000000040008000000000000020'), 'Spam',                'Publicité, lien non sollicité ou contenu répétitif.', 0),
    (UNHEX('00000000000040008000000000000021'), 'Contenu inapproprié', 'Contenu choquant, illégal ou contraire aux règles.',  1),
    (UNHEX('00000000000040008000000000000022'), 'Hors-sujet',          'Ne concerne pas le sujet ou le forum.',               2),
    (UNHEX('00000000000040008000000000000023'), 'Harcèlement',         'Attaque personnelle, intimidation ou insulte.',       3);
