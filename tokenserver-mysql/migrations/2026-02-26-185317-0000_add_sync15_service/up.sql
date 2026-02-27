-- The standard Sync service entry
INSERT IGNORE INTO services (service, pattern) VALUES
    ('sync-1.5', '{node}/1.5/{uid}');
