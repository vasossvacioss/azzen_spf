import psycopg2
def sync():
    conn = psycopg2.connect(host='localhost', port=5432, dbname='spf_excel', user='postgres', password='admin123')
    cur = conn.cursor()
    cur.execute("TRUNCATE TABLE expediente_historial, expediente_agentes, expedientes RESTART IDENTITY CASCADE;")
    conn.commit()
    cur.execute("SELECT COUNT(*) FROM expedientes;")
    count = cur.fetchone()[0]
    print(f"    [OK] Expedientes sincronizados: {count}")
    conn.close()
if __name__ == '__main__':
    sync()