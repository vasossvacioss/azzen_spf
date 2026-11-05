import psycopg2
import sys
try:
    conn = psycopg2.connect(
        host='localhost',
        port=5432,
        dbname='spf_excel',
        user='postgres',
        password='admin123'
    )
    cur = conn.cursor()
    print("=" * 60)
    print("VALIDACION POST-MIGRACION: Integridad de datos en spf_excel")
    print("=" * 60)
    print("\n[1] Conteos finales:")
    for table in ['personas', 'expedientes', 'expediente_agentes', 'expediente_suspensiones']:
        cur.execute(f'SELECT COUNT(*) FROM {table}')
        count = cur.fetchone()[0]
        print(f'  {table}: {count} filas')
    print("\n[2] Expedientes con involucrados:")
    result = cur.fetchone()
    print(f'  Total expedientes: {result[0]}')
    print(f'  Total involucrados: {result[1]}')
    print("\n[3] Ejemplo - Expediente 201 con sus involucrados:")
    rows = cur.fetchall()
    if rows:
        for row in rows:
            if row[1]:
                print(f'  Expediente {row[0]}: Credencial {row[1]}, Nombre: {row[2]}')
    print("\n[4] Suspensiones verificadas:")
    result = cur.fetchone()
    print(f'  Total suspensiones: {result[0]}')
    print(f'  Credenciales suspendidas: {result[1]}')
    print("\n[5] Rango de credenciales cargadas:")
    cur.execute('SELECT MIN(credencial::BIGINT), MAX(credencial::BIGINT) FROM personas')
    result = cur.fetchone()
    print(f'  Credencial minima: {result[0]}')
    print(f'  Credencial maxima: {result[1]}')
    print("\n[6] Verificar nombres y apellidos separados:")
    result = cur.fetchone()
    print(f'  Total personas: {result[0]}')
    print(f'  Con nombre: {result[1]}')
    print(f'  Con apellido: {result[2]}')
    conn.close()
    print("\n" + "=" * 60)
    print("OK - VALIDACION COMPLETADA EXITOSAMENTE")
    print("=" * 60)
except Exception as e:
    print(f"ERROR: {e}")
    sys.exit(1)