# Redis / redis-cli command reference

Quick reference for everyday work with [redis-cli](https://redis.io/docs/ui/cli/). Commands run in the CLI unless noted.

Redis has **logical databases** `0`–`15` (default config); switch with `SELECT`. Keys are flat strings, not tables.

## Connect

```bash
# Local, no auth (default Docker setup in this repo)
redis-cli -h localhost -p 6379

# URI
redis-cli -u redis://localhost:6379/0
redis-cli -u redis://:password@localhost:6379/0
redis-cli -u redis://user:password@localhost:6379/0

# One-shot command
redis-cli -h localhost -p 6379 PING
redis-cli -h localhost -p 6379 GET mykey
```

Inside `redis-cli`, optional auth:

```bash
AUTH password
AUTH username password    # ACL (Redis 6+)
```

## Logical databases

```bash
SELECT 0          # switch DB (0 default)
SELECT 1
DBSIZE            # key count in current DB
FLUSHDB           # ⚠ delete all keys in current DB
FLUSHALL          # ⚠ delete all keys in all DBs
MOVE key 1        # move key to DB 1
```

## Strings

```bash
SET user:1:name "Ada"
GET user:1:name
SET key "value" EX 3600          # expire in seconds
SET key "value" PX 3600000       # expire in milliseconds
SET key "value" NX               # only if not exists
SET key "value" XX               # only if exists
MSET k1 v1 k2 v2
MGET k1 k2
INCR counter
INCRBY counter 5
DECR counter
APPEND key "suffix"
STRLEN key
GETSET key "new"                 # set and return old value
```

## Hashes

```bash
HSET user:1 name "Ada" email "ada@example.com" age 30
HGET user:1 name
HMGET user:1 name email
HGETALL user:1
HDEL user:1 age
HEXISTS user:1 email
HKEYS user:1
HVALS user:1
HLEN user:1
HINCRBY user:1 loginCount 1
```

## Lists

```bash
LPUSH queue:jobs "job-1" "job-2"
RPUSH queue:jobs "job-3"
LPOP queue:jobs
RPOP queue:jobs
LRANGE queue:jobs 0 -1           # all elements
LLEN queue:jobs
LINDEX queue:jobs 0
LSET queue:jobs 0 "updated"
BLPOP queue:jobs 10               # blocking pop (timeout seconds)
```

## Sets

```bash
SADD tags:article:1 "redis" "database" "cache"
SMEMBERS tags:article:1
SISMEMBER tags:article:1 "redis"
SREM tags:article:1 "cache"
SCARD tags:article:1
SINTER set1 set2                  # intersection
SUNION set1 set2
SDIFF set1 set2
```

## Sorted sets

```bash
ZADD leaderboard 100 "player1" 85 "player2" 120 "player3"
ZRANGE leaderboard 0 -1 WITHSCORES
ZREVRANGE leaderboard 0 9 WITHSCORES   # top 10
ZRANK leaderboard "player1"
ZSCORE leaderboard "player1"
ZINCRBY leaderboard 10 "player2"
ZREM leaderboard "player1"
ZCOUNT leaderboard 90 110
```

## Keys (inspect & manage)

```bash
KEYS user:*                      # ⚠ avoid in production (blocks)
SCAN 0 MATCH user:* COUNT 100    # preferred iteration
EXISTS user:1
TYPE user:1
TTL user:1                       # seconds until expiry (-1 no expiry, -2 missing)
PTTL user:1
EXPIRE user:1 3600
EXPIREAT user:1 1735689600      # Unix timestamp
PERSIST user:1                   # remove expiry
DEL user:1 key2 key3
UNLINK user:1                    # async delete (Redis 4+)
RENAME oldkey newkey
RENAMENX oldkey newkey
DUMP key                         # serialized value
```

### SCAN loop (pattern)

```bash
# In redis-cli, repeat with cursor from previous SCAN until cursor 0
SCAN 0 MATCH session:* COUNT 50
```

## Find / “select” data

Redis has no SQL. Typical patterns:

```bash
# Known key
GET session:abc123
HGETALL user:1

# Pattern (dev/small datasets only)
KEYS user:*

# Production: SCAN + GET/HGETALL per key
SCAN 0 MATCH user:* COUNT 100
```

## Transactions

```bash
MULTI
SET account:1:balance 100
DECRBY account:1:balance 10
EXEC

# Discard queued commands
DISCARD

# Optimistic locking
WATCH account:1:balance
# read balance, compute new value in app
MULTI
SET account:1:balance 90
EXEC                           # fails if key changed after WATCH
```

## Pub/Sub

```bash
# Subscriber terminal
SUBSCRIBE notifications
PSUBSCRIBE events:*

# Publisher terminal
PUBLISH notifications "hello"
PUBLISH events:orders "created"
```

## Server & monitoring

```bash
PING                           # PONG
INFO
INFO memory
INFO stats
INFO keyspace
CONFIG GET maxmemory
CONFIG SET maxmemory 256mb       # runtime; may need redis.conf for persist
CLIENT LIST
CLIENT GETNAME
SLOWLOG GET 10
MONITOR                        # ⚠ streams all commands; dev only
TIME
DBSIZE
```

## Persistence (awareness)

```bash
SAVE                           # blocking snapshot
BGSAVE                         # background snapshot
LASTSAVE
```

Docker Compose in this repo runs with `--appendonly yes` (AOF enabled).

## ACL / users (Redis 6+)

```bash
ACL LIST
ACL WHOAMI
ACL SETUSER appuser on >secret ~* +@read +@write
ACL DELUSER appuser
```

## Useful redis-cli modes

```bash
redis-cli --stat                 # live stats
redis-cli --bigkeys              # sample large keys
redis-cli --latency
redis-cli --intrinsic-latency 100
redis-cli --csv KEYS 'user:*'    # machine-readable (still avoid KEYS in prod)
```

## Docker (this repo)

From project root:

```bash
make compose_up
redis-cli -h localhost -p 6379
```

Default Redis service has **no password** (`deploy/docker-compose.yml`). Use `SELECT` for DB `0`–`15` as needed.

## Tips

- Prefer **`SCAN`** over **`KEYS`** on large datasets.
- Key naming: `object:id:field` (e.g. `user:42:profile`).
- Set **TTL** on cache/session keys (`EX` / `EXPIRE`).
- **`FLUSHDB` / `FLUSHALL`** are destructive; double-check `SELECT` first.
- Logical DB index in URI: `redis://localhost:6379/2` → `SELECT 2`.
- For pgconnect: Redis connections use DB number in the “database” field and `redis-cli -u` when connecting.
