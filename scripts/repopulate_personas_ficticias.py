import os
import sqlite3
import psycopg2
from datetime import datetime
SQLITE_PATH = os.path.join(os.path.dirname(__file__), "..", "excel_archivos_sinteticos", "azzen_spf_demo.sqlite")
PG_HOST = "localhost"
PG_PORT = 5432
PG_DB = "spf_excel"
PG_USER = "postgres"
PG_PASS = "admin123"
def main():
    print("=" * 60)
    print("REPOBLACIÓN Y SANEAMIENTO DE PERSONAS CON DATOS FICTICIOS SPF")
    print("=" * 60)
    if not os.path.exists(SQLITE_PATH):
        raise FileNotFoundError(f"No se encontró la base SQLite sintética en: {SQLITE_PATH}")
    sq_conn = sqlite3.connect(SQLITE_PATH)
    sq_cur = sq_conn.cursor()
    sq_personas = sq_cur.fetchall()
    print(f"[OK] Se cargaron {len(sq_personas)} personas ficticias desde SQLite.")
    pg_conn = psycopg2.connect(
        host=PG_HOST,
        port=PG_PORT,
        dbname=PG_DB,
        user=PG_USER,
        password=PG_PASS
    )
    pg_cur = pg_conn.cursor()
    pg_cur.execute("SELECT DISTINCT credencial FROM expediente_agentes;")
    ea_creds = set(r[0] for r in pg_cur.fetchall())
    print(f"[i] Credenciales con expedientes/sumarios en expediente_agentes: {len(ea_creds)}")
    pg_cur.execute("SELECT DISTINCT credencial FROM expediente_suspensiones;")
    es_creds = set(r[0] for r in pg_cur.fetchall())
    print(f"[i] Credenciales con suspensiones en expediente_suspensiones: {len(es_creds)}")
    pg_cur.execute("SELECT COUNT(*) FROM personas;")
    prev_count = pg_cur.fetchone()[0]
    print(f"[!] Cantidad previa en personas (datos a depurar): {prev_count}")
    pg_cur.execute("DELETE FROM personas;")
    print("[OK] Tabla 'personas' vaciada satisfactoriamente.")
    now = datetime.now()
    inserted_count = 0
    for row in sq_personas:
        cred = str(row[0]).strip()
        nom = (row[1] or "").strip()
        ap = (row[2] or "").strip()
        nombre_apellido = f"{ap}, {nom}" if ap and nom else (nom or ap or f"Agente {cred}")
        cargo = (row[6] or "").strip()
        if cargo == "Analista":
            escalafon = "Profesional"
            grado = "Analista"
        elif cargo in ["Agente", "Suboficial"]:
            escalafon = "Seguridad"
            grado = cargo
        elif cargo in ["Oficial Principal", "Inspector"]:
            escalafon = "Seguridad"
            grado = cargo
        else:
            escalafon = "Seguridad"
            grado = cargo or "Agente"
        escalafon_grado = f"{escalafon} - {grado}"
        if cred in ea_creds:
            origen_principal = "sumarios_caratulas"
        elif cred in es_creds:
            origen_principal = "suspensiones"
        else:
            origen_principal = "personas_padron"
        fecha_ingreso_raw = row[8]
        if fecha_ingreso_raw:
            try:
                first_seen_at = datetime.strptime(fecha_ingreso_raw, "%Y-%m-%d")
            except Exception:
                first_seen_at = datetime(2020, 1, 1, 8, 0, 0)
        else:
            first_seen_at = datetime(2020, 1, 1, 8, 0, 0)
        pg_cur.execute(insert_sql, (
            cred,
            ap,
            nom,
            nombre_apellido,
            escalafon,
            grado,
            escalafon_grado,
            origen_principal,
            first_seen_at,
            now
        ))
        inserted_count += 1
    print(f"[OK] Se insertaron {inserted_count} agentes ficticios en 'personas'.")
    agentes_updated = pg_cur.rowcount
    print(f"[OK] Se actualizaron {agentes_updated} filas en 'expediente_agentes' con escalafón y grado normalizados.")
    pg_conn.commit()
    pg_cur.execute("SELECT COUNT(*) FROM personas;")
    final_count = pg_cur.fetchone()[0]
    orig_breakdown = pg_cur.fetchall()
    samples = pg_cur.fetchall()
    print("\n" + "=" * 60)
    print("RESUMEN DE SANEAMIENTO:")
    print(f"Total personas final: {final_count}")
    print(f"Desglose por origen principal: {orig_breakdown}")
    print("Muestra de personas insertadas:")
    for s in samples:
        print(f"  - Cred: {s[0]} | Apellido: {s[1]} | Nombre: {s[2]} | Grado: {s[3]} | Origen: {s[4]}")
    print("=" * 60)
    sq_conn.close()
    pg_conn.close()
if __name__ == "__main__":
    main()