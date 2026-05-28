INSERT OR IGNORE INTO postgres_connections
(name, host, port, database, username, encrypted_password, nonce)
SELECT name, host, port, database, username, encrypted_password, nonce
FROM connections
WHERE db_type = 'postgres';

INSERT OR IGNORE INTO mongodb_connections
(name, host, port, database, auth_source, mongo_uri, username, encrypted_password, nonce)
SELECT name, host, port, database, auth_source, mongo_uri, username, encrypted_password, nonce
FROM connections
WHERE db_type = 'mongodb';

INSERT OR IGNORE INTO redis_connections
(name, host, port, database, username, encrypted_password, nonce)
SELECT name, host, port, database, username, encrypted_password, nonce
FROM connections
WHERE db_type = 'redis';
