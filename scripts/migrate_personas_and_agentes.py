import psycopg2
import openpyxl
import os
conn = psycopg2.connect('postgres://postgres:admin123@localhost:5432/spf_excel')
cur = conn.cursor()
print("1. Migrando esquema de personas...")
print("2. Migrando esquema de expediente_agentes...")
conn.commit()
xlsx_path = r'C:\Users\javie\OneDrive\Documentos\personas.xlsx'
if os.path.exists(xlsx_path):
    print("3. Actualizando padrón de personas desde personas.xlsx...")
    wb = openpyxl.load_workbook(xlsx_path, read_only=True)
    ws = wb.active
    rows = list(ws.iter_rows(values_only=True))
    header = [str(h).strip().lower() if h else '' for h in rows[0]]
    print("Excel header:", header)
    c_idx = header.index('credencial') if 'credencial' in header else 0
    nom_idx = -1
    ape_idx = -1
    esc_idx = -1
    for i, h in enumerate(header):
        if 'nombre' in h: nom_idx = i
        elif 'apellido' in h: ape_idx = i
        elif 'escalafon' in h: esc_idx = i
    updated = 0
    inserted = 0
    for r in rows[1:]:
        cred = str(r[c_idx]).strip() if r[c_idx] is not None else ''
        if not cred: continue
        nom = str(r[nom_idx]).strip() if nom_idx >= 0 and r[nom_idx] is not None else ''
        ape = str(r[ape_idx]).strip() if ape_idx >= 0 and r[ape_idx] is not None else ''
        esc_raw = str(r[esc_idx]).strip() if esc_idx >= 0 and r[esc_idx] is not None else ''
        esc = esc_raw.strip('() ')
        grado = ''
        if '-' in esc:
            parts = esc.split('-', 1)
            esc = parts[0].strip()
            grado = parts[1].strip()
        elif '/' in esc:
            parts = esc.split('/', 1)
            esc = parts[0].strip()
            grado = parts[1].strip()
        updated += 1
    conn.commit()
    print(f"Pobladas/actualizadas {updated} personas desde Excel.")
print("4. Normalizando expediente_agentes existentes...")
updated_ea = cur.rowcount
print(f"Sincronizados {updated_ea} agentes de expedientes con el padrón de Personas.")
remaining = cur.fetchall()
print(f"Agentes restantes por separar nombre_completo: {len(remaining)}")
for ag_id, nc in remaining:
    if ',' in nc:
        parts = nc.split(',', 1)
        ape = parts[0].strip()
        nom = parts[1].strip()
    elif ' ' in nc:
        parts = nc.rsplit(' ', 1)
        nom = parts[0].strip()
        ape = parts[1].strip()
    else:
        ape = nc.strip()
        nom = ''
    cur.execute("UPDATE expediente_agentes SET apellido = %s, nombre = %s WHERE id = %s", (ape, nom, ag_id))
conn.commit()
print("Verificación de datos en personas:")
cur.execute("SELECT credencial, apellido, nombre, escalafon, grado FROM personas WHERE escalafon IS NOT NULL LIMIT 5;")
for r in cur.fetchall():
    print(" ", r)
print("Verificación de datos en expediente_agentes:")
cur.execute("SELECT id, credencial, apellido, nombre, escalafon, grado FROM expediente_agentes LIMIT 5;")
for r in cur.fetchall():
    print(" ", r)
conn.close()
print("¡Migración completada con éxito!")