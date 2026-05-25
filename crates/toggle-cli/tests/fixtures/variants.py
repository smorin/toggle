"""Fixture for variant section tests."""

# toggle:start ID=db:sqlite desc="SQLite backend"
import sqlite3
conn = sqlite3.connect("app.db")
# toggle:end ID=db:sqlite

# toggle:start ID=db:postgres desc="Postgres backend"
# import psycopg2
# conn = psycopg2.connect("host=localhost")
# toggle:end ID=db:postgres

# toggle:start ID=debug
print("debug enabled")
# toggle:end ID=debug

# toggle:start ID=cache:redis
import redis
# toggle:end ID=cache:redis

# toggle:start ID=cache:memcached
# import memcache
# toggle:end ID=cache:memcached

# toggle:start ID=cache:inmemory
# cache = {}
# toggle:end ID=cache:inmemory
