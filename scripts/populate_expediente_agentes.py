import psycopg2
conn = psycopg2.connect("postgres://postgres:admin123@localhost:5432/spf_excel")
cur = conn.cursor()
print("Populating expediente_agentes from sumarios...")
inserted = cur.rowcount
conn.commit()
print(f"Inserted {inserted} rows into expediente_agentes.")
cur.execute("SELECT count(*) FROM expediente_agentes")
print("Total rows in expediente_agentes:", cur.fetchone()[0])
print("Expedientes for 23033:", cur.fetchall())