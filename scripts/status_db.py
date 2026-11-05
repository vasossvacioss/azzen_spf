import psycopg2
def print_status():
    try:
        conn = psycopg2.connect(host='localhost', port=5432, dbname='spf_excel', user='postgres', password='admin123')
        cur = conn.cursor()
        cur.execute("SELECT COUNT(*) FROM source_files;")
        files = cur.fetchone()[0]
        cur.execute("SELECT COUNT(*) FROM source_sheets;")
        sheets = cur.fetchone()[0]
        cur.execute("SELECT COUNT(*) FROM raw_sheet_rows;")
        rows = cur.fetchone()[0]
        cur.execute("SELECT COUNT(*) FROM row_credential_index;")
        creds = cur.fetchone()[0]
        cur.execute("SELECT COUNT(*) FROM expedientes;")
        exps = cur.fetchone()[0]
        cur.execute("SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='public' AND table_name LIKE 'tbl_sheet_%';")
        tbls = cur.fetchone()[0]
        print(f"  Archivos registrados:       {files}")
        print(f"  Hojas registradas:          {sheets} ({tbls} tablas físicas)")
        print(f"  Filas importadas totales:   {rows}")
        print(f"  Credenciales indexadas:     {creds}")
        print(f"  Expedientes sincronizados:  {exps}")
        conn.close()
    except Exception as e:
        print(f"  [!] Error al consultar base de datos: {e}")
if __name__ == '__main__':
    print_status()