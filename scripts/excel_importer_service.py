import os
import sys
import re
import json
from datetime import datetime, date
from pathlib import Path
import openpyxl
import psycopg2
from psycopg2.extras import RealDictCursor
DB_DSN = os.getenv("DATABASE_URL", "postgres://postgres:admin123@localhost:5432/spf_excel")
PATTERNS = {
    "expediente": [
        r"^(?:n[°ºo\.]?\s*)?(?:expediente|expte|actuaci[oó]n|sumario|sumarial)",
        r"numero\s+(?:de\s+)?(?:expediente|sumario|actuacion)",
        r"^expediente$",
        r"^sumario$"
    ],
    "tipo": [
        r"^tipo(?:\s+identificador|\s+de\s+actuaci[oó]n)?$"
    ],
    "reparticion": [
        r"^repartici[oó]n",
        r"^dependencia",
        r"^destino",
        r"^unidad"
    ],
    "motivo": [
        r"^motivo",
        r"^car[aá]tula",
        r"^causa$",
        r"^objeto"
    ],
    "estado": [
        r"^estado",
        r"^situaci[oó]n(?:\s+actual)?"
    ],
    "instructor": [
        r"^instructor(?:\s+designado|\s+actual)?",
        r"^instructor_actual"
    ],
    "instructor_credencial": [
        r"^(?:credencial|legajo)\s+(?:del?\s+)?instructor"
    ],
    "fecha_inicio": [
        r"^fecha(?:\s+de)?\s+inicio",
        r"^fecha(?:\s+de)?\s+ingreso",
        r"^fecha$"
    ],
    "causa_judicial": [
        r"^causa\s+judicial"
    ]
}
def clean_str(val):
    if val is None:
        return ""
    if isinstance(val, (datetime, date)):
        return val.strftime("%Y-%m-%d")
    s = str(val).strip()
    s = re.sub(r"\s+", " ", s)
    return s
def normalize_credencial(val):
    if val is None:
        return ""
    s = str(val).strip()
    if re.match(r"^\d+\.0$", s):
        s = s[:-2]
    s = s.replace(".", "").replace(",", "").strip()
    m = re.search(r"\b([A-Za-z0-9]+)\b", s)
    if m:
        return m.group(1).upper()
    return s.upper()
def classify_header(header_text):
    Detecta columnas horizontales para agentes:
    Ej: [Agente 1, Credencial 1, Agente 2, Credencial 2...]
    o [Involucrado 1, Cred 1, Involucrado 2, Cred 2...]
    Analiza un archivo Excel y devuelve reporte estructurado sin modificar la BD.
    Ejecuta la importación efectiva y atómica en PostgreSQL:
    - Agrega nuevas personas al padrón central (tabla personas) con credenciales intactas
    - Inserta expedientes en tabla expedientes
    - Inserta vínculos en expediente_agentes sin duplicaciones
    - Registra auditoría
            CREATE UNIQUE INDEX IF NOT EXISTS idx_expediente_agentes_unique_rel 
            ON expediente_agentes (expediente_id, credencial) 
            WHERE credencial IS NOT NULL;
                        INSERT INTO expedientes (numero_expediente, tipo_identificador, reparticion, motivo, estado, instructor_actual, fecha_inicio, created_at, updated_at)
                        VALUES (%s, %s, %s, %s, %s, %s, %s, NOW(), NOW())
                        RETURNING id
                            INSERT INTO personas (credencial, nombre_apellido, apellido, nombre, first_seen_at, last_seen_at, origen_principal)
                            VALUES (%s, %s, %s, %s, NOW(), NOW(), 'importacion_excel')
                            ON CONFLICT (credencial) DO NOTHING
                        INSERT INTO expediente_agentes (expediente_id, credencial, nombre_completo, rol, situacion_procesal, created_at, updated_at)
                        VALUES (%s, %s, %s, 'Causante', 'En trámite', NOW(), NOW())
                        ON CONFLICT (expediente_id, credencial) DO NOTHING
                        RETURNING id
            INSERT INTO app_audit_log (action, entity, details, created_at)
            VALUES ('import_excel_commit', 'expedientes', %s, NOW())