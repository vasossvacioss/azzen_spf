from __future__ import annotations
import sqlite3
from pathlib import Path
import psycopg2
from psycopg2 import sql
ROOT = Path(__file__).resolve().parents[1]
SQLITE_DB = ROOT / "excel_archivos_sinteticos" / "azzen_spf_demo.sqlite"
PG_HOST = "localhost"
PG_PORT = 5432
PG_USER = "postgres"
PG_PASSWORD = "admin123"
PG_DB = "spf_demo"
def ensure_database() -> None:
    admin = psycopg2.connect(
        host=PG_HOST,
        port=PG_PORT,
        dbname="postgres",
        user=PG_USER,
        password=PG_PASSWORD,
    )
    admin.autocommit = True
    with admin.cursor() as cur:
        cur.execute("SELECT 1 FROM pg_database WHERE datname = %s", (PG_DB,))
        if cur.fetchone() is None:
            cur.execute(sql.SQL("CREATE DATABASE {}" ).format(sql.Identifier(PG_DB)))
    admin.close()
def build_postgres_schema(conn) -> None:
    with conn.cursor() as cur:
        cur.execute(ddl)
    conn.commit()
def load_sqlite_to_postgres() -> None:
    with sqlite3.connect(SQLITE_DB) as sqlite_conn:
        tables = [
            "personas",
            "expedientes",
            "involucrados_expediente",
            "historial_credencial",
            "suspensiones",
            "sumarios",
            "actuaciones",
        ]
        for table in tables:
            columns = [row[1] for row in sqlite_conn.execute(f"PRAGMA table_info('{table}')").fetchall()]
            rows = sqlite_conn.execute(f"SELECT * FROM {table}").fetchall()
            if not rows:
                continue
            placeholders = ", ".join(["%s"] * len(columns))
            insert_sql = f"INSERT INTO {table} ({', '.join(columns)}) VALUES ({placeholders})"
            with psycopg2.connect(
                host=PG_HOST,
                port=PG_PORT,
                dbname=PG_DB,
                user=PG_USER,
                password=PG_PASSWORD,
            ) as pg_conn:
                with pg_conn.cursor() as cur:
                    cur.execute(f"TRUNCATE TABLE {table} RESTART IDENTITY CASCADE;")
                    cur.executemany(insert_sql, rows)
                pg_conn.commit()
            print(f"Imported {len(rows)} rows to {table}")
def main() -> None:
    ensure_database()
    conn = psycopg2.connect(
        host=PG_HOST,
        port=PG_PORT,
        dbname=PG_DB,
        user=PG_USER,
        password=PG_PASSWORD,
    )
    try:
        build_postgres_schema(conn)
        conn.commit()
        load_sqlite_to_postgres()
    finally:
        conn.close()
    print(f"PostgreSQL demo DB ready: {PG_DB}")
    print(f"SQLite source: {SQLITE_DB}")
    print("Views created: vista_expediente_involucrados, vista_historial_credencial")
if __name__ == "__main__":
    main()