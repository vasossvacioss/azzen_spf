from __future__ import annotations
import sqlite3
from pathlib import Path
from datetime import datetime
import psycopg2
from psycopg2 import sql
ROOT = Path(__file__).resolve().parents[1]
SQLITE_DB = ROOT / "excel_archivos_sinteticos" / "azzen_spf_demo.sqlite"
BACKUP_DIR = ROOT / "backup_pre_migracion_sintetica"
PG_HOST = "localhost"
PG_PORT = 5432
PG_USER = "postgres"
PG_PASSWORD = "admin123"
PG_DB = "spf_excel"
def backup_postgres() -> Path:
    BACKUP_DIR.mkdir(parents=True, exist_ok=True)
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    backup_file = BACKUP_DIR / f"spf_excel_backup_{timestamp}.sql"
    conn = psycopg2.connect(
        host=PG_HOST,
        port=PG_PORT,
        dbname=PG_DB,
        user=PG_USER,
        password=PG_PASSWORD,
    )
    with conn.cursor() as cur:
        tables = [row[0] for row in cur.fetchall()]
    conn.close()
    sql_dump = []
    for table in tables:
        conn = psycopg2.connect(
            host=PG_HOST,
            port=PG_PORT,
            dbname=PG_DB,
            user=PG_USER,
            password=PG_PASSWORD,
        )
        with conn.cursor() as cur:
            cur.execute(f"SELECT * FROM {table}")
            rows = cur.fetchall()
            col_names = [desc[0] for desc in cur.description]
            if rows:
                cols_str = ", ".join(col_names)
                for row in rows:
                    val_strs = []
                    for val in row:
                        if val is None:
                            val_strs.append("NULL")
                        elif isinstance(val, str):
                            val_strs.append(f"'{val.replace(chr(39), chr(39)*2)}'")
                        else:
                            val_strs.append(str(val))
                    vals_str = ", ".join(val_strs)
                    sql_dump.append(f"INSERT INTO {table} ({cols_str}) VALUES ({vals_str});")
        conn.close()
    backup_file.write_text("\n".join(sql_dump), encoding="utf-8")
    print(f"[BACKUP] Respaldo guardado: {backup_file}")
    return backup_file
def truncate_tables(conn) -> None:
    with conn.cursor() as cur:
        tables = [row[0] for row in cur.fetchall()]
        for table in tables:
            try:
                cur.execute(f"TRUNCATE TABLE {table} RESTART IDENTITY CASCADE")
                print(f"[LIMPIAR] Truncada tabla: {table}")
            except Exception as e:
                print(f"[ADVERTENCIA] No se pudo truncar {table}: {e}")
    conn.commit()
def load_sqlite_to_postgres() -> None:
    sqlite_conn = sqlite3.connect(SQLITE_DB)
    pg_conn = psycopg2.connect(
        host=PG_HOST,
        port=PG_PORT,
        dbname=PG_DB,
        user=PG_USER,
        password=PG_PASSWORD,
    )
    print("\n[MIGRAR] Tabla: personas")
    cursor = sqlite_conn.execute("SELECT credencial_numero, nombre, apellido, nombre_completo, documento, genero, cargo, destino, fecha_ingreso FROM personas")
    personas_rows = cursor.fetchall()
    for row in personas_rows:
        credencial_numero, nombre, apellido, nombre_completo, documento, genero, cargo, destino, fecha_ingreso = row
        with pg_conn.cursor() as cur:
                str(credencial_numero),
                nombre or '',
                apellido or '',
                nombre_completo or '',
                'demo_sintetico'
            ))
        pg_conn.commit()
    print(f"  {len(personas_rows)} filas")
    print("[MIGRAR] Tabla: expedientes")
    cursor = sqlite_conn.execute("SELECT expediente_numero, sumario_numero, fecha_inicio, fecha_cierre, estado, tipo_actuacion, resolucion, prioridad FROM expedientes")
    expedientes_rows = cursor.fetchall()
    for row in expedientes_rows:
        expediente_numero, sumario_numero, fecha_inicio, fecha_cierre, estado, tipo_actuacion, resolucion, prioridad = row
        with pg_conn.cursor() as cur:
                'sumario',
                str(expediente_numero),
                'Demo Sintético',
                tipo_actuacion or 'Sumario',
                fecha_inicio or None,
                estado or 'Abierto',
                f'Instructor_Demo_{sumario_numero or ""}'
            ))
        pg_conn.commit()
    print(f"  {len(expedientes_rows)} filas")
    print("[MIGRAR] Tabla: expediente_agentes (desde involucrados_expediente)")
    involucrados_rows = cursor.fetchall()
    for expediente_numero, credencial_numero, rol, nombre_completo in involucrados_rows:
        with pg_conn.cursor() as cur:
            result = cur.fetchone()
            if result:
                expediente_id = result[0]
                    expediente_id,
                    str(credencial_numero),
                    nombre_completo or f'Agente {credencial_numero}',
                    rol or 'Involucrado',
                    'Regular'
                ))
        pg_conn.commit()
    print(f"  {len(involucrados_rows)} filas")
    print("[MIGRAR] Tabla: expediente_suspensiones (desde suspensiones)")
    suspensiones_rows = cursor.fetchall()
    for credencial_numero, expediente_numero, fecha_desde, fecha_hasta, estado_suspension, motivo, nombre_completo in suspensiones_rows:
        with pg_conn.cursor() as cur:
            result = cur.fetchone()
            if result:
                expediente_id = result[0]
                agent_result = cur.fetchone()
                agente_id = agent_result[0] if agent_result else None
                    expediente_id,
                    agente_id,
                    str(credencial_numero),
                    nombre_completo or f'Agente {credencial_numero}',
                    fecha_desde,
                    estado_suspension or 'Vigente',
                    'sistema_migracion',
                    f'Motivo: {motivo}' if motivo else None
                ))
        pg_conn.commit()
    print(f"  {len(suspensiones_rows)} filas")
    sqlite_conn.close()
    pg_conn.close()
def verify_migration() -> None:
    conn = psycopg2.connect(
        host=PG_HOST,
        port=PG_PORT,
        dbname=PG_DB,
        user=PG_USER,
        password=PG_PASSWORD,
    )
    print("\n[VALIDACION] Conteos finales en spf_excel:")
    with conn.cursor() as cur:
        for table in ["personas", "expedientes", "expediente_agentes", "expediente_suspensiones"]:
            cur.execute(f"SELECT COUNT(*) FROM {table}")
            count = cur.fetchone()[0]
            print(f"  {table}: {count} filas")
    conn.close()
def main() -> None:
    print("=" * 80)
    print("MIGRACIÓN DE DATOS SINTÉTICOS A BASE REAL: spf_excel")
    print("=" * 80)
    print("\n[1/4] Creando respaldo de seguridad...")
    backup_path = backup_postgres()
    print("\n[2/4] Limpiando tablas existentes en spf_excel...")
    conn = psycopg2.connect(
        host=PG_HOST,
        port=PG_PORT,
        dbname=PG_DB,
        user=PG_USER,
        password=PG_PASSWORD,
    )
    truncate_tables(conn)
    conn.close()
    print("\n[3/4] Migrando datos sintéticos desde SQLite...")
    load_sqlite_to_postgres()
    print("\n[4/4] Validando migración...")
    verify_migration()
    print("\n" + "=" * 80)
    print("MIGRACIÓN COMPLETADA EXITOSAMENTE")
    print(f"Respaldo guardado en: {backup_path}")
    print(f"Base de datos: {PG_DB}")
    print("=" * 80)
if __name__ == "__main__":
    main()