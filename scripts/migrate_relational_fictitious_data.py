from __future__ import annotations
import sqlite3
from pathlib import Path
from typing import Any
from openpyxl import Workbook, load_workbook
ROOT = Path(__file__).resolve().parents[1]
TARGET_DIR = ROOT / "excel_archivos_sinteticos"
DB_PATH = TARGET_DIR / "azzen_spf_demo.sqlite"
SCHEMA_PATH = TARGET_DIR / "azzen_spf_relational_schema.sql"
def normalize_header(value: Any) -> str:
    text = str(value or "").strip()
    text = text.lower()
    text = text.replace("n°", "numero ")
    text = text.replace("nº", "numero ")
    text = text.replace("nro.", "numero ")
    text = text.replace("\n", " ")
    text = text.replace("_", " ")
    replacements = {
        ",": " ",
        "/": " ",
        "(": " ",
        ")": " ",
        "-": " ",
    }
    for old, new in replacements.items():
        text = text.replace(old, new)
    text = " ".join(text.split())
    text = text.replace("  ", " ")
    return text
def canonicalize_name(alias: str) -> str:
    clean = normalize_header(alias)
    aliases = {
        "credencial": "credencial_numero",
        "cred": "credencial_numero",
        "numero de credencial": "credencial_numero",
        "nro de credencial": "credencial_numero",
        "numero expediente": "expediente_numero",
        "nro expediente": "expediente_numero",
        "numero de expediente": "expediente_numero",
        "sumario": "sumario_numero",
        "sumario numero": "sumario_numero",
        "nombre": "nombre",
        "nombres": "nombre",
        "apellido": "apellido",
        "apellidos": "apellido",
        "apellido nombres": "nombre_completo",
        "nombre apellido": "nombre_completo",
        "nombre y apellido": "nombre_completo",
        "apellido nombre": "nombre_completo",
        "apellido y nombre": "nombre_completo",
        "fecha desde": "fecha_desde",
        "desde": "fecha_desde",
        "fecha hasta": "fecha_hasta",
        "hasta": "fecha_hasta",
        "fecha vencimiento 60 dias": "fecha_vencimiento_60",
        "fecha vencimiento 120 dias": "fecha_vencimiento_120",
        "estado": "estado",
        "motivo": "motivo",
        "observaciones": "observaciones",
        "resolucion": "resolucion",
        "destino": "destino",
        "tipo actuacion": "tipo_actuacion",
    }
    return aliases.get(clean, clean.replace(" ", "_"))
def split_name(value: Any) -> tuple[str, str]:
    text = (value or "").strip()
    if not text:
        return "", ""
    text = text.replace("  ", " ")
    if "," in text:
        left, right = [part.strip() for part in text.split(",", 1)]
        return (right or left, left or right)
    parts = text.split()
    if len(parts) <= 1:
        return text, ""
    if len(parts) == 2:
        return parts[0], parts[1]
    return " ".join(parts[:-1]), parts[-1]
def ensure_demo_directory() -> None:
    TARGET_DIR.mkdir(parents=True, exist_ok=True)
def create_schema(connection: sqlite3.Connection) -> None:
    connection.executescript(schema)
    SCHEMA_PATH.write_text(schema, encoding="utf-8")
def load_table_from_workbook(connection: sqlite3.Connection, workbook_path: Path, sheet_name: str, table_name: str, columns: list[str]) -> None:
    wb = load_workbook(workbook_path, data_only=True)
    ws = wb[sheet_name]
    rows = list(ws.iter_rows(values_only=True))
    if not rows:
        return
    header_row = rows[0]
    normalized = [canonicalize_name(str(h)) for h in header_row]
    mapping = {header: normalized_index for header, normalized_index in zip(header_row, range(len(header_row)))}
    for row in rows[1:]:
        payload = {}
        for idx, key in enumerate(header_row):
            canonical = canonicalize_name(str(key))
            payload[canonical] = row[idx]
        if table_name == "personas":
            if "nombre_completo" in payload and not payload.get("nombre") and not payload.get("apellido"):
                nombre, apellido = split_name(payload.get("nombre_completo"))
                payload["nombre"] = nombre
                payload["apellido"] = apellido
            nombre = payload.get("nombre") or ""
            apellido = payload.get("apellido") or ""
            if isinstance(nombre, str) and isinstance(apellido, str):
                nombre_completo = f"{nombre} {apellido}".strip()
            else:
                nombre_completo = ""
            connection.execute(
                "INSERT INTO personas (credencial_numero, nombre, apellido, nombre_completo, documento, genero, cargo, destino, fecha_ingreso) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    int(payload.get("credencial_numero") or 0),
                    str(payload.get("nombre") or nombre or "").strip(),
                    str(payload.get("apellido") or apellido or "").strip(),
                    nombre_completo,
                    int(payload.get("documento") or 0) if str(payload.get("documento") or "").strip() else None,
                    payload.get("genero"),
                    payload.get("cargo"),
                    payload.get("destino"),
                    payload.get("fecha_ingreso"),
                ),
            )
        elif table_name == "expedientes":
            connection.execute(
                "INSERT INTO expedientes (expediente_numero, sumario_numero, fecha_inicio, fecha_cierre, estado, tipo_actuacion, resolucion, prioridad) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    int(payload.get("expediente_numero") or 0),
                    int(payload.get("sumario_numero") or 0) if str(payload.get("sumario_numero") or "").strip() else None,
                    payload.get("fecha_inicio"),
                    payload.get("fecha_cierre"),
                    payload.get("estado"),
                    payload.get("tipo_actuacion"),
                    payload.get("resolucion"),
                    payload.get("prioridad"),
                ),
            )
        elif table_name == "involucrados_expediente":
            connection.execute(
                "INSERT INTO involucrados_expediente (expediente_numero, credencial_numero, rol, tipo_involucrado, participacion, estado_vinculo) VALUES (?, ?, ?, ?, ?, ?)",
                (
                    int(payload.get("expediente_numero") or 0),
                    int(payload.get("credencial_numero") or 0),
                    payload.get("rol"),
                    payload.get("tipo_involucrado"),
                    payload.get("participacion"),
                    payload.get("estado_vinculo"),
                ),
            )
        elif table_name == "historial_credencial":
            connection.execute(
                "INSERT INTO historial_credencial (credencial_numero, expediente_numero, fecha_evento, evento, estado_evento, observaciones) VALUES (?, ?, ?, ?, ?, ?)",
                (
                    int(payload.get("credencial_numero") or 0),
                    int(payload.get("expediente_numero") or 0),
                    payload.get("fecha_evento"),
                    payload.get("evento"),
                    payload.get("estado_evento"),
                    payload.get("observaciones"),
                ),
            )
        elif table_name == "suspensiones":
            connection.execute(
                "INSERT INTO suspensiones (credencial_numero, expediente_numero, fecha_desde, fecha_hasta, estado_suspension, motivo, resolucion, fecha_vencimiento_60, fecha_vencimiento_120) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    int(payload.get("credencial_numero") or 0),
                    int(payload.get("expediente_numero") or 0),
                    payload.get("fecha_desde"),
                    payload.get("fecha_hasta"),
                    payload.get("estado_suspension"),
                    payload.get("motivo"),
                    payload.get("resolucion"),
                    payload.get("fecha_vencimiento_60"),
                    payload.get("fecha_vencimiento_120"),
                ),
            )
        elif table_name == "sumarios":
            connection.execute(
                "INSERT INTO sumarios (sumario_numero, expediente_numero, credencial_numero, tipo_sumario, instructor, motivo, estado_sumario, observaciones) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    int(payload.get("sumario_numero") or 0),
                    int(payload.get("expediente_numero") or 0),
                    int(payload.get("credencial_numero") or 0),
                    payload.get("tipo_sumario"),
                    payload.get("instructor"),
                    payload.get("motivo"),
                    payload.get("estado_sumario"),
                    payload.get("observaciones"),
                ),
            )
        elif table_name == "actuaciones":
            connection.execute(
                "INSERT INTO actuaciones (expediente_numero, credencial_numero, fecha_actuacion, tipo_actuacion, responsable, resultado, observaciones) VALUES (?, ?, ?, ?, ?, ?, ?)",
                (
                    int(payload.get("expediente_numero") or 0),
                    int(payload.get("credencial_numero") or 0),
                    payload.get("fecha_actuacion"),
                    payload.get("tipo_actuacion"),
                    payload.get("responsable"),
                    payload.get("resultado"),
                    payload.get("observaciones"),
                ),
            )
    connection.commit()
def import_generated_workbooks(connection: sqlite3.Connection) -> None:
    workbook_specs = [
        (TARGET_DIR / "AZZEN_SPF_estructura_normalizada.xlsx", "personas", "personas"),
        (TARGET_DIR / "AZZEN_SPF_estructura_normalizada.xlsx", "expedientes", "expedientes"),
        (TARGET_DIR / "AZZEN_SPF_datos_ficticios_01.xlsx", "involucrados_expediente", "involucrados_expediente"),
        (TARGET_DIR / "AZZEN_SPF_datos_ficticios_01.xlsx", "historial_credencial", "historial_credencial"),
        (TARGET_DIR / "AZZEN_SPF_datos_ficticios_02.xlsx", "suspensiones", "suspensiones"),
        (TARGET_DIR / "AZZEN_SPF_datos_ficticios_02.xlsx", "sumarios", "sumarios"),
        (TARGET_DIR / "AZZEN_SPF_datos_ficticios_03.xlsx", "actuaciones", "actuaciones"),
        (TARGET_DIR / "AZZEN_SPF_datos_ficticios_03.xlsx", "credenciales_historico", "personas"),
    ]
    for workbook_path, sheet_name, table_name in workbook_specs:
        if not workbook_path.exists():
            continue
        try:
            load_table_from_workbook(connection, workbook_path, sheet_name, table_name, [])
        except Exception:
            continue
    connection.execute(
        "DELETE FROM personas WHERE rowid NOT IN (SELECT MIN(rowid) FROM personas GROUP BY credencial_numero)"
    )
    connection.execute(
        "UPDATE personas SET nombre_completo = TRIM(COALESCE(nombre, '') || ' ' || COALESCE(apellido, '')) WHERE nombre_completo IS NULL OR TRIM(nombre_completo) = ''"
    )
    connection.commit()
def main() -> None:
    ensure_demo_directory()
    connection = sqlite3.connect(DB_PATH)
    create_schema(connection)
    import_generated_workbooks(connection)
    connection.close()
    print(f"DB creada: {DB_PATH}")
    print(f"Schema SQL: {SCHEMA_PATH}")
if __name__ == "__main__":
    main()