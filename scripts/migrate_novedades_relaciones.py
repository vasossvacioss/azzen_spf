import psycopg2
def run_migration():
    conn = psycopg2.connect("postgresql://postgres:admin123@localhost:5432/spf_excel")
    cur = conn.cursor()
    print("Iniciando migraciones para Novedades, Suspensiones y Relaciones...")
    statements = [
        "CREATE INDEX IF NOT EXISTS idx_novedades_expediente ON expediente_novedades(expediente_id);",
        "CREATE INDEX IF NOT EXISTS idx_novedades_agente ON expediente_novedades(agente_id);",
        "CREATE INDEX IF NOT EXISTS idx_novedades_fecha ON expediente_novedades(fecha_evento);",
        "CREATE INDEX IF NOT EXISTS idx_novedades_tipo ON expediente_novedades(tipo_novedad);",
        "CREATE INDEX IF NOT EXISTS idx_suspensiones_exp ON expediente_suspensiones(expediente_id);",
        "CREATE INDEX IF NOT EXISTS idx_suspensiones_cred ON expediente_suspensiones(credencial);",
        "CREATE INDEX IF NOT EXISTS idx_suspensiones_estado ON expediente_suspensiones(estado);",
        "CREATE INDEX IF NOT EXISTS idx_relaciones_origen ON expediente_relaciones(expediente_origen_id);",
        "CREATE INDEX IF NOT EXISTS idx_relaciones_destino ON expediente_relaciones(expediente_destino_id);",
        "CREATE INDEX IF NOT EXISTS idx_relaciones_tipo ON expediente_relaciones(tipo_relacion);",
        "CREATE INDEX IF NOT EXISTS idx_documentos_exp ON expediente_documentos(expediente_id);",
        "CREATE INDEX IF NOT EXISTS idx_documentos_nov ON expediente_documentos(novedad_id);",
    ]
    for stmt in statements:
        cur.execute(stmt)
    conn.commit()
    print("Migración completada exitosamente.")
    conn.close()
if __name__ == "__main__":
    run_migration()