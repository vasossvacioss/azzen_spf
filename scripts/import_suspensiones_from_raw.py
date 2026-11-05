import psycopg2
import json
import re
from datetime import datetime
def parse_date(val):
    if not val or val == "Sin datos" or val.startswith("-") or val.startswith("="):
        return None
    val_str = str(val).strip()
    m = re.search(r'(\d{4})-(\d{2})-(\d{2})', val_str)
    if m:
        try:
            return datetime(int(m.group(1)), int(m.group(2)), int(m.group(3)))
        except:
            pass
    m = re.search(r'(\d{1,2})/(\d{1,2})/(\d{4})', val_str)
    if m:
        try:
            return datetime(int(m.group(3)), int(m.group(2)), int(m.group(1)))
        except:
            pass
    return None
def extract_incisos(text):
    if not text:
        return "a)"
    t = text.lower()
    found = []
    for inc in ['a', 'b', 'c', 'd', 'e']:
        if f'inc. {inc}' in t or f'inc.{inc}' in t or f'inciso {inc}' in t or f'incisos {inc}' in t or f'inc. {inc})' in t or f'{inc})' in t:
            found.append(f"{inc})")
    if found:
        return ", ".join(dict.fromkeys(found))
    return "a)"
def run_import():
    conn = psycopg2.connect("postgresql://postgres:admin123@localhost:5432/spf_excel")
    cur = conn.cursor()
    cur.execute("SELECT id, numero_expediente FROM expedientes")
    exps = cur.fetchall()
    print(f"Total expedientes cargados para matching: {len(exps)}")
    rows = cur.fetchall()
    print(f"Filas de suspensiones a procesar: {len(rows)}")
    inserted_count = 0
    for row_id, sheet_id, payload in rows:
        cred = str(payload.get('credencial', '')).strip().replace('.', '')
        if not cred or cred == 'Sin datos' or cred == 'None':
            continue
        nombres = str(payload.get('nombres', '')).strip()
        apellidos = str(payload.get('apellido_nombres', payload.get('apellidos', ''))).strip()
        nombre_completo = f"{apellidos} {nombres}".strip()
        if not nombre_completo:
            nombre_completo = f"Agente Cred. {cred}"
        desde_val = payload.get('desde')
        fecha_inicio = parse_date(desde_val) or datetime(2020, 1, 1)
        encuadre = str(payload.get('resoluc_dn_orden_interna_encuadre_rrd', ''))
        incisos = extract_incisos(encuadre)
        sumario_txt = str(payload.get('sumario_numero_instructor_motivo', ''))
        obs = str(payload.get('observaciones', ''))
        mantenimiento = str(payload.get('mantenimiento', ''))
        levantamiento_txt = str(payload.get('levantamiento', ''))
        expediente_id = None
        for eid, eno in exps:
            if eno and len(eno) > 5 and (eno in sumario_txt or eno in encuadre or eno in obs):
                expediente_id = eid
                break
        existing = cur.fetchone()
        estado = 'Vigente' if sheet_id == 1 else 'Levantada'
        if sheet_id == 2:
            fecha_fin = parse_date(obs) or parse_date(mantenimiento) or datetime(2025, 1, 1)
            motivo_fin = 'Levantamiento dispuesto por resolución'
            fundamento_fin = f"{levantamiento_txt} {obs} {mantenimiento}".strip()
        else:
            fecha_fin = None
            motivo_fin = None
            fundamento_fin = None
        full_obs = f"{mantenimiento} | {obs} | {sumario_txt}".strip(" |")
        if not existing:
                expediente_id, cred, nombre_completo, fecha_inicio,
                incisos, estado, fecha_fin, motivo_fin,
                fundamento_fin, 'Importación Inicial', full_obs[:1000]
            ))
            susp_id = cur.fetchone()[0]
            inserted_count += 1
            if expediente_id:
                    expediente_id, susp_id,
                    'Suspensión preventiva dispuesta' if estado == 'Vigente' else 'Levantamiento de suspensión',
                    fecha_inicio,
                    f"Suspensión preventiva: {nombre_completo} (Cred. {cred})",
                    f"Art. 419 incisos: {incisos}. {encuadre[:200]}",
                    'Importación Inicial', False
                ))
    conn.commit()
    print(f"Total suspensiones importadas: {inserted_count}")
    conn.close()
if __name__ == "__main__":
    run_import()