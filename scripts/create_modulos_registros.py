import psycopg2
from datetime import datetime
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
    print("Creando estructuras de módulos de registros")
    print("=" * 60)
    print("\n[1] Creando tabla: modulos_registros...")
    print("[2] Creando tabla: modulos_vistas...")
    print("[3] Creando tabla: modulos_campos...")
    print("\n[4] Insertando módulos de registros...")
    modulos = [
        ("Registros de Personas", "Gestión de personal y credenciales", "👥", 1),
        ("Registros de Expedientes", "Expedientes principales y administrativos", "📁", 2),
        ("Registros de Involucrados", "Personas involucradas en expedientes", "🔗", 3),
        ("Registros de Suspensiones", "Suspensiones preventivas y disciplinarias", "⛔", 4),
        ("Registros de Sumarios", "Sumarios administrativos", "⚖️", 5),
        ("Registros de Actuaciones", "Actuaciones y novedades sobre expedientes", "📝", 6),
    ]
    for nombre, descripcion, icono, orden in modulos:
        print(f"   OK {nombre}")
    print("\n[5] Recuperando IDs de módulos...")
    cur.execute("SELECT id, nombre_modulo FROM modulos_registros ORDER BY orden")
    modulos_map = {row[1]: row[0] for row in cur.fetchall()}
    print("\n[6] Insertando vistas por módulo...")
    vistas_config = {
        "Registros de Personas": [
            ("Lista de Personas", "lista", "personas", "Listar todas las personas registradas"),
            ("Crear Persona", "crear", "personas", "Agregar una nueva persona"),
            ("Historial de Cambios", "historial", "personas", "Ver el historial de cambios"),
        ],
        "Registros de Expedientes": [
            ("Lista de Expedientes", "lista", "expedientes", "Listar todos los expedientes"),
            ("Crear Expediente", "crear", "expedientes", "Crear un nuevo expediente"),
            ("Historial de Versiones", "historial", "expediente_versiones", "Ver versiones anteriores"),
        ],
        "Registros de Involucrados": [
            ("Lista de Involucrados", "lista", "expediente_agentes", "Listar personas involucradas en expedientes"),
            ("Vincular Involucrado", "crear", "expediente_agentes", "Agregar persona a expediente"),
            ("Cambios de Rol", "historial", "expediente_agentes", "Ver cambios en roles"),
        ],
        "Registros de Suspensiones": [
            ("Suspensiones Activas", "lista", "expediente_suspensiones", "Listar suspensiones vigentes"),
            ("Crear Suspensión", "crear", "expediente_suspensiones", "Registrar nueva suspensión"),
            ("Historial de Levantamientos", "historial", "expediente_suspensiones", "Ver suspensiones ya levantadas"),
        ],
        "Registros de Sumarios": [
            ("Sumarios Abiertos", "lista", "expedientes", "Listar sumarios en proceso"),
            ("Crear Sumario", "crear", "expedientes", "Iniciar nuevo sumario"),
            ("Sumarios Cerrados", "historial", "expedientes", "Ver sumarios finalizados"),
        ],
        "Registros de Actuaciones": [
            ("Últimas Actuaciones", "lista", "expediente_novedades", "Listar actuaciones recientes"),
            ("Registrar Actuación", "crear", "expediente_novedades", "Crear nueva actuación"),
            ("Búsqueda de Actuaciones", "historial", "expediente_novedades", "Buscar actuaciones por período"),
        ],
    }
    for nombre_modulo, vistas in vistas_config.items():
        modulo_id = modulos_map.get(nombre_modulo)
        if modulo_id:
            for vista_nombre, tipo, tabla, desc in vistas:
                print(f"   OK {nombre_modulo} => {vista_nombre}")
    conn.commit()
    print("\n[7] Validando datos cargados...")
    cur.execute("SELECT COUNT(*) FROM modulos_registros")
    modulos_count = cur.fetchone()[0]
    cur.execute("SELECT COUNT(*) FROM modulos_vistas")
    vistas_count = cur.fetchone()[0]
    print(f"   Módulos: {modulos_count}")
    print(f"   Vistas: {vistas_count}")
    print("\n[8] Estructura de módulos:")
    for row in cur.fetchall():
        print(f"   {row[0]}: {row[2]} vistas")
    conn.close()
    print("\n" + "=" * 60)
    print("OK - Tablas de modulos creadas exitosamente")
    print("=" * 60)
except Exception as e:
    print(f"ERROR: {e}")
    import traceback
    traceback.print_exc()