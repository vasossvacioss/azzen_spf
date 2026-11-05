import os
import sys
import re
import json
import hashlib
import random
import psycopg2
import openpyxl
from faker import Faker
import builtins
orig_print = builtins.print
def print(*args, **kwargs):
    kwargs.setdefault('flush', True)
    orig_print(*args, **kwargs)
builtins.print = print
def get_connection():
    return psycopg2.connect(host='localhost', port=5432, dbname='spf_excel', user='postgres', password='admin123')
def build_anonymizer():
    print("=" * 80)
    print(" AZZEN SPF — MOTOR DE ANONIMIZACIÓN INTEGRAL Y GENERACIÓN SINTÉTICA ")
    print("=" * 80)
    random.seed(20261031)
    fake = Faker('es_AR')
    Faker.seed(20261031)
    conn = get_connection()
    cur = conn.cursor()
    cur.execute("SELECT current_database(), current_user, inet_server_addr();")
    db_info = cur.fetchone()
    print(f"[SEGURIDAD] Conectado a DB: {db_info[0]} | Usuario: {db_info[1]} | Host: {db_info[2] or 'localhost'}")
    assert db_info[0] == 'spf_excel', "¡ERROR! La base de datos no es spf_excel."
    backup_path = "c:\\Proyectos\\SPF\\backup_pre_anonimizacion\\spf_excel_pre_anonymize.dump"
    if not os.path.exists(backup_path):
        raise RuntimeError("No se encontró el backup de seguridad en " + backup_path)
    print(f"[SEGURIDAD] Copia de seguridad verificada: {backup_path} ({os.path.getsize(backup_path)} bytes)")
    print("\n--- ETAPA 1: Reconocimiento de identidades, credenciales y expedientes ---")
    all_creds = set()
    cur.execute("SELECT DISTINCT credencial FROM personas WHERE credencial IS NOT NULL AND credencial != '';")
    all_creds.update(r[0].strip() for r in cur.fetchall())
    cur.execute("SELECT DISTINCT credencial FROM expediente_agentes WHERE credencial IS NOT NULL AND credencial != '';")
    all_creds.update(r[0].strip() for r in cur.fetchall())
    cur.execute("SELECT DISTINCT credencial FROM expediente_suspensiones WHERE credencial IS NOT NULL AND credencial != '';")
    all_creds.update(r[0].strip() for r in cur.fetchall())
    cur.execute("SELECT DISTINCT instructor_credencial FROM expedientes WHERE instructor_credencial IS NOT NULL AND instructor_credencial != '';")
    all_creds.update(r[0].strip() for r in cur.fetchall())
    cur.execute("SELECT DISTINCT credential_value FROM row_credential_index WHERE credential_value IS NOT NULL AND credential_value != '';")
    all_creds.update(r[0].strip() for r in cur.fetchall())
    for s_id in range(1, 15):
        try:
            cur.execute(f"SELECT table_name FROM source_sheets WHERE id = {s_id};")
            trow = cur.fetchone()
            if not trow: continue
            tname = trow[0]
            cur.execute(f"SELECT column_name FROM information_schema.columns WHERE table_name='{tname}' AND column_name LIKE '%credencial%';")
            ccols = [r[0] for r in cur.fetchall()]
            for ccol in ccols:
                cur.execute(f'SELECT DISTINCT "{ccol}" FROM "{tname}" WHERE "{ccol}" IS NOT NULL AND "{ccol}" != \'\';')
                all_creds.update(r[0].strip() for r in cur.fetchall())
        except Exception:
            pass
    sorted_creds = sorted(list(all_creds))
    print(f"Total de credenciales únicas identificadas: {len(sorted_creds)}")
    cur.execute("SELECT credencial, apellido, nombre, nombre_apellido FROM personas;")
    persona_info = {r[0].strip(): (r[1] or '', r[2] or '', r[3] or '') for r in cur.fetchall()}
    cur.execute("SELECT DISTINCT numero_expediente FROM expedientes WHERE numero_expediente IS NOT NULL AND numero_expediente != '';")
    all_exps = sorted(list(set(r[0].strip() for r in cur.fetchall())))
    print(f"Total de expedientes únicos identificados: {len(all_exps)}")
    print("\n--- ETAPA 2: Construcción de diccionario sintético ---")
    cred_map = {}
    person_map = {}
    name_replacements = []
    apellidos_pool = [
        "Salvatierra", "Giménez", "Navarro", "Balmaceda", "Quiroga", "Paredes", "Cáceres", 
        "Benítez", "Albarracín", "Villalba", "Ferreyra", "Santillán", "Vega", "Correa", 
        "Montenegro", "Mansilla", "Maldonado", "Chávez", "Escobar", "Aguirre", "Medina", 
        "Bustos", "Ríos", "Luna", "Acuña", "Carrizo", "Peralta", "Godoy", "Sosa", "Lucero",
        "Juárez", "Barrios", "Castillo", "Ibáñez", "Cardozo", "Romero", "Franco", "Ledesma",
        "Toledo", "Figueroa", "Vera", "Bravo", "Arce", "Valdez", "Guzmán", "Palacios", 
        "Roldán", "Villanueva", "Gallardo", "Barreto", "Paz", "Ochoa", "Morales", "Herrera"
    ]
    nombres_pool_m = [
        "Federico Martín", "Carlos Alberto", "Lucas Matías", "Diego Hernán", "Mariano José",
        "Facundo Nahuel", "Gonzalo Damián", "Esteban Daniel", "Javier Alejandro", "Nicolás Andrés",
        "Cristian Ariel", "Rodrigo Ezequiel", "Gabriel Maximiliano", "Sebastián Ignacio", "Emiliano Gastón",
        "Mauro Luciano", "Pablo Andrés", "Fernando Gabriel", "Ignacio Tomás", "Leandro Agustín",
        "Matías Eduardo", "Germán Horacio", "Víctor Manuel", "Rubén Darío", "Héctor Ramón"
    ]
    nombres_pool_f = [
        "Valeria Soledad", "Romina Giselle", "Camila Belén", "Florencia Andrea", "Luciana Beatriz",
        "Mariana Elizabeth", "Silvina Paola", "Claudia Marcela", "Jésica Natalia", "Laura Patricia",
        "Natalia Carolina", "Paula Daniela", "Gabriela Fernanda", "Carla Yanina", "Sabrina Noemí"
    ]
    all_nombres = nombres_pool_m + nombres_pool_f
    preserved_creds = {"70000", "99991", "99999", "admin", "e2e_val_admin", "e2e_val_bloqueado"}
    synth_cred_counter = 870001
    for c in sorted_creds:
        if c in preserved_creds:
            s_cred = c
        else:
            s_cred = str(synth_cred_counter)
            synth_cred_counter += 1
        cred_map[c] = s_cred
        ap = random.choice(apellidos_pool)
        nom = random.choice(all_nombres)
        full_synth = f"{ap}, {nom}"
        orig_ap, orig_nom, orig_full = persona_info.get(c, ('', '', ''))
        person_map[c] = {
            "synth_cred": s_cred,
            "synth_apellido": ap,
            "synth_nombre": nom,
            "synth_nombre_completo": full_synth,
            "synth_apellido_upper": ap.upper(),
            "synth_nombre_upper": nom.upper(),
            "synth_nombre_completo_upper": full_synth.upper(),
            "orig_cred": c,
            "orig_apellido": orig_ap,
            "orig_nombre": orig_nom,
            "orig_full": orig_full
        }
        if orig_full and len(orig_full) >= 4:
            name_replacements.append((orig_full.strip(), full_synth))
            name_replacements.append((orig_full.upper().strip(), full_synth.upper()))
        if orig_ap and len(orig_ap) >= 4 and orig_nom and len(orig_nom) >= 3:
            name_replacements.append((f"{orig_nom} {orig_ap}", f"{nom} {ap}"))
            name_replacements.append((f"{orig_nom.upper()} {orig_ap.upper()}", f"{nom.upper()} {ap.upper()}"))
            name_replacements.append((f"{orig_ap} {orig_nom}", f"{ap} {nom}"))
            name_replacements.append((f"{orig_ap.upper()} {orig_nom.upper()}", f"{ap.upper()} {nom.upper()}"))
    name_replacements.sort(key=lambda x: len(x[0]), reverse=True)
    seen_orig = set()
    dedup_name_replacements = []
    for o, s in name_replacements:
        if o not in seen_orig and len(o) >= 4:
            seen_orig.add(o)
            dedup_name_replacements.append((o, s))
    name_replacements = dedup_name_replacements
    print(f"Diccionario de credenciales generado: {len(cred_map)} asignaciones.")
    print(f"Diccionario de reemplazos nominales: {len(name_replacements)} patrones.")
    exp_map = {}
    synth_exp_counter = 10001
    for exp in all_exps:
        m_year = re.search(r'\b(19\d\d|20\d\d)\b', exp)
        year = m_year.group(1) if m_year else "2026"
        if "APN" in exp or exp.startswith("EX-"):
            s_exp = f"EX-{year}-8{synth_exp_counter:07d}- -APN-DEMO
        elif "CUDAP" in exp or "EXP-S04" in exp:
            s_exp = f"CUDAP: EXP-S04: 008{synth_exp_counter:04d}/{year}"
        elif "/" in exp and re.match(r'^\d+/\d+$', exp.strip()):
            s_exp = f"80{synth_exp_counter:03d}/{year}"
        elif exp in ["Sin datos", "En trámite", "Inicio", "Reservado"]:
            s_exp = exp
        else:
            s_exp = f"SPF-DEMO-{year}-{synth_exp_counter:05d}"
        exp_map[exp] = s_exp
        synth_exp_counter += 1
    print(f"Diccionario de expedientes generado: {len(exp_map)} asignaciones.")
    cred_patterns = []
    for orig_c, s_c in cred_map.items():
        if orig_c in preserved_creds: continue
        if orig_c.isdigit() and len(orig_c) in [4, 5]:
            c_dotted = f"{orig_c[:-3]}.{orig_c[-3:]}"
            cred_patterns.append((re.compile(r'\b' + re.escape(c_dotted) + r'\b'), s_c))
        cred_patterns.append((re.compile(r'\b' + re.escape(orig_c) + r'\b'), s_c))
    re_gde = re.compile(r'\b(NO|IF|PV|DI|RESOL|EX)[-_](\d{4})[-_](\d+)[-_]APN[-_]([A-Z0-9
    re_dni = re.compile(r'\b(?:DNI|D\.N\.I\.|Documento)?\s*(?:N[°ºo\.]*)?\s*(\d{1,2}\.?\d{3}\.?\d{3})\b')
    re_email = re.compile(r'[a-zA-Z0-9_.+-]+@[a-zA-Z0-9-]+\.[a-zA-Z0-9-.]+')
    re_phone = re.compile(r'(?:\+?54\s*)?(?:9\s*)?(?:11|\d{2,4})[\s\-\.]?\d{3,4}[\s\-\.]?\d{4}')
    re_lpu = re.compile(r'\bL\.?P\.?U\.?\s*(\d[\d\.\/A-Z]*)\b', re.IGNORECASE)
    def sanitize_free_text(text):
        if not text or not isinstance(text, str):
            return text
        res = text
        for o_name, s_name in name_replacements:
            if o_name in res:
                res = res.replace(o_name, s_name)
        for pat, s_c in cred_patterns:
            res = pat.sub(s_c, res)
        res = re_gde.sub(r'\1-\2-88800001-APN-DEMO
        res = re_dni.sub('DNI Nº 98.765.432', res)
        res = re_lpu.sub('L.P.U. 999.888/D', res)
        res = re_email.sub('demo.usuario@spf.gob.ar', res)
        res = re_phone.sub('(011) 5555-0100', res)
        return res
    def sanitize_json_val(val):
        if isinstance(val, dict):
            new_d = {}
            for k, v in val.items():
                k_lower = k.lower()
                if "credencial" in k_lower and isinstance(v, str) and v.strip() in cred_map:
                    new_d[k] = cred_map[v.strip()]
                elif "expediente" in k_lower and isinstance(v, str) and v.strip() in exp_map:
                    new_d[k] = exp_map[v.strip()]
                else:
                    new_d[k] = sanitize_json_val(v)
            return new_d
        elif isinstance(val, list):
            return [sanitize_json_val(x) for x in val]
        elif isinstance(val, str):
            return sanitize_free_text(val)
        else:
            return val
    print("\n--- ETAPA 3: Aplicando transformaciones en PostgreSQL ---")
    print("  -> Actualizando tabla personas...")
    cur.execute("SELECT credencial, apellido, nombre FROM personas;")
    p_rows = cur.fetchall()
    for orig_c, ap, nom in p_rows:
        orig_c_s = orig_c.strip() if orig_c else ""
        if orig_c_s in cred_map:
            p_data = person_map[orig_c_s]
            s_c = p_data["synth_cred"]
            s_ap = p_data["synth_apellido"]
            s_nom = p_data["synth_nombre"]
            s_full = p_data["synth_nombre_completo"]
    conn.commit()
    print("     [OK] personas actualizadas.")
    print("  -> Actualizando tabla expediente_agentes...")
    cur.execute("SELECT id, credencial, cargos_articulos FROM expediente_agentes;")
    ea_rows = cur.fetchall()
    for eid, c, cargos in ea_rows:
        c_s = (c or "").strip()
        s_c = cred_map.get(c_s, c_s)
        p_data = person_map.get(c_s)
        s_full = p_data["synth_nombre_completo"] if p_data else "Agente Sintético SPF"
        s_cargos = sanitize_free_text(cargos)
    conn.commit()
    print("     [OK] expediente_agentes actualizados.")
    print("  -> Actualizando tabla expediente_suspensiones...")
    cur.execute("SELECT id, credencial, motivo_fin, fundamento_fin, observaciones FROM expediente_suspensiones;")
    es_rows = cur.fetchall()
    for sid, c, m_fin, f_fin, obs in es_rows:
        c_s = (c or "").strip()
        s_c = cred_map.get(c_s, c_s)
        p_data = person_map.get(c_s)
        s_full = p_data["synth_nombre_completo"] if p_data else "Agente Sintético SPF"
    conn.commit()
    print("     [OK] expediente_suspensiones actualizadas.")
    print("  -> Actualizando tabla expedientes...")
    cur.execute("SELECT id, numero_expediente, caratula, motivo, relato_hechos, instructor_actual, instructor_credencial, causa_judicial, observaciones FROM expedientes;")
    exp_rows = cur.fetchall()
    for exp_id, n_exp, car, mot, rel, inst, inst_c, causa_jud, obs in exp_rows:
        s_exp = exp_map.get((n_exp or "").strip(), n_exp)
        s_car = sanitize_free_text(car)
        s_mot = sanitize_free_text(mot)
        s_rel = sanitize_free_text(rel)
        s_causa = sanitize_free_text(causa_jud)
        s_obs = sanitize_free_text(obs)
        inst_c_s = (inst_c or "").strip()
        s_inst_c = cred_map.get(inst_c_s, inst_c_s)
        p_inst = person_map.get(inst_c_s)
        s_inst = p_inst["synth_nombre_completo"] if p_inst else sanitize_free_text(inst)
    conn.commit()
    print("     [OK] expedientes actualizados.")
    print("  -> Actualizando tabla row_credential_index...")
    cur.execute("SELECT id, credential_value FROM row_credential_index;")
    rci_rows = cur.fetchall()
    for rci_id, val in rci_rows:
        val_s = (val or "").strip()
        s_val = cred_map.get(val_s, val_s)
        if s_val != val:
            cur.execute("UPDATE row_credential_index SET credential_value = %s WHERE id = %s;", (s_val, rci_id))
    conn.commit()
    print("     [OK] row_credential_index actualizado.")
    print("  -> Actualizando tabla app_fila_novedades...")
    cur.execute("SELECT id, credencial, numero_expediente, titulo, descripcion, observaciones FROM app_fila_novedades;")
    afn_rows = cur.fetchall()
    for fn_id, c, exp, tit, desc, obs in afn_rows:
        c_s = (c or "").strip()
        s_c = cred_map.get(c_s, c_s)
        p_data = person_map.get(c_s)
        s_name = p_data["synth_nombre_completo"] if p_data else "Agente Sintético SPF"
        s_exp = exp_map.get((exp or "").strip(), exp)
    conn.commit()
    print("     [OK] app_fila_novedades actualizadas.")
    print("  -> Actualizando tabla expediente_novedades...")
    cur.execute("SELECT id, titulo, descripcion, observaciones FROM expediente_novedades;")
    en_rows = cur.fetchall()
    for en_id, tit, desc, obs in en_rows:
    conn.commit()
    print("     [OK] expediente_novedades actualizadas.")
    print("  -> Actualizando tabla expediente_versiones...")
    cur.execute("SELECT id, snapshot_json, motivo FROM expediente_versiones;")
    ev_rows = cur.fetchall()
    for ev_id, snap, mot in ev_rows:
        s_snap = sanitize_json_val(snap)
        s_mot = sanitize_free_text(mot)
        s_hash = hashlib.sha256(json.dumps(s_snap, sort_keys=True).encode('utf-8')).hexdigest()
    conn.commit()
    print("     [OK] expediente_versiones actualizadas.")
    print("  -> Actualizando tabla raw_sheet_rows (6619 filas)...")
    cur.execute("SELECT id, payload_json FROM raw_sheet_rows;")
    rsr_rows = cur.fetchall()
    for rsr_id, payload in rsr_rows:
        s_payload = sanitize_json_val(payload)
        cur.execute("UPDATE raw_sheet_rows SET payload_json = %s WHERE id = %s;", (json.dumps(s_payload), rsr_id))
    conn.commit()
    print("     [OK] raw_sheet_rows actualizadas.")
    cur.execute("SELECT id, snapshot_fila, titulo, descripcion, observaciones FROM raw_sheet_row_versiones;")
    rsrv_rows = cur.fetchall()
    for rsrv_id, snap, tit, desc, obs in rsrv_rows:
        s_snap = sanitize_json_val(snap)
    conn.commit()
    print("     [OK] raw_sheet_row_versiones actualizadas.")
    print("  -> Actualizando identidades demo en app_users...")
    conn.commit()
    print("     [OK] app_users sanitizado.")
    print("  -> Actualizando calidad de datos, auditoría y notificaciones...")
    cur.execute("SELECT id, identificador_visible, descripcion, sugerencia FROM app_calidad_datos_casos;")
    for cid, ivis, desc, sug in cur.fetchall():
    cur.execute("SELECT id, details FROM app_audit_log;")
    for aid, det in cur.fetchall():
        cur.execute("UPDATE app_audit_log SET details = %s WHERE id = %s;", (json.dumps(sanitize_json_val(det)), aid))
    cur.execute("SELECT id, titulo, mensaje, agente_credencial FROM app_notificaciones;")
    for nid, tit, msg, acre in cur.fetchall():
        s_acre = cred_map.get((acre or "").strip(), acre)
    conn.commit()
    print("     [OK] Calidad, auditoría y notificaciones actualizadas.")
    print("  -> Actualizando tablas físicas de hojas tbl_sheet_1 a tbl_sheet_14...")
    for s_id in range(1, 15):
        cur.execute(f"SELECT table_name FROM source_sheets WHERE id = {s_id};")
        trow = cur.fetchone()
        if not trow: continue
        tname = trow[0]
        cur.execute(f"SELECT column_name, data_type FROM information_schema.columns WHERE table_name='{tname}' AND column_name NOT IN ('id', 'run_id', 'source_row_number');")
        cols = cur.fetchall()
        col_names = [c[0] for c in cols]
        select_cols = ", ".join(f'"{c}"' for c in col_names)
        cur.execute(f'SELECT id, {select_cols} FROM "{tname}";')
        sheet_rows = cur.fetchall()
        for s_row in sheet_rows:
            row_id = s_row[0]
            updates = []
            params = []
            for idx, cname in enumerate(col_names):
                val = s_row[idx + 1]
                if val is None: continue
                cname_lower = cname.lower()
                if cname == "payload_json":
                    new_val = json.dumps(sanitize_json_val(val))
                elif "credencial" in cname_lower and isinstance(val, str) and val.strip() in cred_map:
                    new_val = cred_map[val.strip()]
                elif ("expediente" in cname_lower or "sumario" in cname_lower) and isinstance(val, str) and val.strip() in exp_map:
                    new_val = exp_map[val.strip()]
                elif any(x in cname_lower for x in ["apellido", "nombre", "causante", "instructor"]) and isinstance(val, str):
                    new_val = sanitize_free_text(val)
                elif isinstance(val, str):
                    new_val = sanitize_free_text(val)
                else:
                    new_val = val
                updates.append(f'"{cname}" = %s')
                params.append(new_val)
            if updates:
                params.append(row_id)
                cur.execute(f'UPDATE "{tname}" SET {", ".join(updates)} WHERE id = %s;', tuple(params))
        conn.commit()
        print(f"     [OK] {tname} ({len(sheet_rows)} filas actualizadas).")
    print("\n--- ETAPA 4: Anonimización de los 4 archivos Excel en excel archivos/ ---")
    excel_dir = "c:\\Proyectos\\SPF\\excel archivos"
    for fname in os.listdir(excel_dir):
        if not fname.endswith(".xlsx"): continue
        fpath = os.path.join(excel_dir, fname)
        print(f"  -> Procesando {fname}...")
        try:
            wb = openpyxl.load_workbook(fpath)
            for sname in wb.sheetnames:
                ws = wb[sname]
                for row in ws.iter_rows():
                    for cell in row:
                        if cell.value and isinstance(cell.value, str):
                            val = cell.value.strip()
                            if val in cred_map:
                                cell.value = cred_map[val]
                            elif val in exp_map:
                                cell.value = exp_map[val]
                            else:
                                cell.value = sanitize_free_text(cell.value)
            wb.save(fpath)
            print(f"     [OK] {fname} guardado con datos ficticios.")
        except Exception as ex:
            print(f"     [!] Advertencia en {fname}: {ex}")
    print("\n--- ETAPA 5: Auditoría y comprobación de datos reales residuales ---")
    sample_real_creds = [c for c in sorted_creds[:50] if c not in preserved_creds]
    found_residual_creds = 0
    for rc in sample_real_creds:
        cur.execute("SELECT count(*) FROM personas WHERE credencial = %s;", (rc,))
        cnt = cur.fetchone()[0]
        if cnt > 0:
            found_residual_creds += 1
            print(f"  [ALERTA] Credencial real encontrada en personas: {rc}")
    print(f"Comprobación de credenciales residuales: {found_residual_creds} encontradas (Esperado: 0).")
    cur.execute("SELECT count(*) FROM personas;")
    total_p = cur.fetchone()[0]
    cur.execute("SELECT count(*) FROM expedientes;")
    total_e = cur.fetchone()[0]
    cur.execute("SELECT count(*) FROM expediente_agentes;")
    total_ea = cur.fetchone()[0]
    cur.execute("SELECT count(*) FROM expediente_suspensiones;")
    total_es = cur.fetchone()[0]
    print(f"\nTotales finales preservados:")
    print(f"  - personas:                {total_p} filas")
    print(f"  - expedientes:             {total_e} filas")
    print(f"  - expediente_agentes:      {total_ea} filas")
    print(f"  - expediente_suspensiones: {total_es} filas")
    conn.close()
    print("\n" + "=" * 80)
    print(" ¡PROCESO DE ANONIMIZACIÓN Y GENERACIÓN SINTÉTICA COMPLETADO CON ÉXITO! ")
    print("=" * 80)
if __name__ == '__main__':
    build_anonymizer()