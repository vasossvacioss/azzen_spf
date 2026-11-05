class ExpedienteItem {
  final int id;
  final String tipoIdentificador;
  final String numeroExpediente;
  final String reparticion;
  final String? motivo;
  final String estado;
  final String? instructorActual;
  final String? fechaInicio;
  final int totalAgentes;
  ExpedienteItem({
    required this.id,
    required this.tipoIdentificador,
    required this.numeroExpediente,
    required this.reparticion,
    this.motivo,
    required this.estado,
    this.instructorActual,
    this.fechaInicio,
    this.totalAgentes = 0,
  });
  factory ExpedienteItem.fromJson(Map<String, dynamic> json) {
    return ExpedienteItem(
      id: json['id'] is int ? json['id'] : int.tryParse(json['id'].toString()) ?? 0,
      tipoIdentificador: json['tipo_identificador']?.toString() ?? 'sumario',
      numeroExpediente: json['numero_expediente']?.toString() ?? 'S/N',
      reparticion: json['reparticion']?.toString() ?? 'General',
      motivo: json['motivo']?.toString() ?? json['caratula']?.toString(),
      estado: json['estado']?.toString() ?? 'Abierto',
      instructorActual: json['instructor_actual']?.toString(),
      fechaInicio: json['fecha_inicio']?.toString(),
      totalAgentes: json['total_agentes'] is int
          ? json['total_agentes']
          : int.tryParse(json['total_agentes']?.toString() ?? '0') ?? 0,
    );
  }
}
class ExpedienteDetalle {
  final int id;
  final String tipoIdentificador;
  final String numeroExpediente;
  final String reparticion;
  final String? fechaInicio;
  final String? fechaHecho;
  final String? fechaPrevistaFin;
  final String? autoridadOrdenante;
  final String? autoridadDependencia;
  final String? motivo;
  final String estado;
  final String? articulo;
  final String? inciso;
  final String? relatoHechos;
  final String? instructorActual;
  final int versionActual;
  final List<AgenteInvolucrado> agentes;
  final List<SuspensionAsociada> suspensiones;
  final List<NovedadExpediente> novedades;
  final List<DocumentoExpediente> documentos;
  final List<VersionExpediente> versiones;
  ExpedienteDetalle({
    required this.id,
    required this.tipoIdentificador,
    required this.numeroExpediente,
    required this.reparticion,
    this.fechaInicio,
    this.fechaHecho,
    this.fechaPrevistaFin,
    this.autoridadOrdenante,
    this.autoridadDependencia,
    this.motivo,
    required this.estado,
    this.articulo,
    this.inciso,
    this.relatoHechos,
    this.instructorActual,
    required this.versionActual,
    this.agentes = const [],
    this.suspensiones = const [],
    this.novedades = const [],
    this.documentos = const [],
    this.versiones = const [],
  });
  factory ExpedienteDetalle.fromJson(Map<String, dynamic> json) {
    final exp = json['expediente'] as Map<String, dynamic>? ?? {};
    final agList = (json['agentes'] as List? ?? []).map((a) => AgenteInvolucrado.fromJson(a)).toList();
    final suspList = (json['suspensiones'] as List? ?? []).map((s) => SuspensionAsociada.fromJson(s)).toList();
    final novList = (json['novedades'] as List? ?? []).map((n) => NovedadExpediente.fromJson(n)).toList();
    final docList = (json['documentos'] as List? ?? []).map((d) => DocumentoExpediente.fromJson(d)).toList();
    final verList = (json['versiones'] as List? ?? []).map((v) => VersionExpediente.fromJson(v)).toList();
    return ExpedienteDetalle(
      id: exp['id'] is int ? exp['id'] : int.tryParse(exp['id']?.toString() ?? '0') ?? 0,
      tipoIdentificador: exp['tipo_identificador']?.toString() ?? 'sumario',
      numeroExpediente: exp['numero_expediente']?.toString() ?? 'S/N',
      reparticion: exp['reparticion']?.toString() ?? 'General',
      fechaInicio: exp['fecha_inicio']?.toString(),
      fechaHecho: exp['fecha_hecho']?.toString(),
      fechaPrevistaFin: exp['fecha_prevista_fin']?.toString(),
      autoridadOrdenante: exp['autoridad_ordenante']?.toString(),
      autoridadDependencia: exp['autoridad_dependencia']?.toString(),
      motivo: exp['motivo']?.toString(),
      estado: exp['estado']?.toString() ?? 'Abierto',
      articulo: exp['articulo']?.toString(),
      inciso: exp['inciso']?.toString(),
      relatoHechos: exp['relato_hechos']?.toString(),
      instructorActual: exp['instructor_actual']?.toString(),
      versionActual: exp['version_actual'] is int ? exp['version_actual'] : 1,
      agentes: agList,
      suspensiones: suspList,
      novedades: novList,
      documentos: docList,
      versiones: verList,
    );
  }
}
class AgenteInvolucrado {
  final int id;
  final String credencial;
  final String nombreCompleto;
  final String rol;
  final String situacionProcesal;
  final bool suspensionPreventiva;
  final bool causaJudicial;
  final String? escalafon;
  final String? grado;
  AgenteInvolucrado({
    required this.id,
    required this.credencial,
    required this.nombreCompleto,
    required this.rol,
    required this.situacionProcesal,
    required this.suspensionPreventiva,
    required this.causaJudicial,
    this.escalafon,
    this.grado,
  });
  factory AgenteInvolucrado.fromJson(Map<String, dynamic> json) {
    return AgenteInvolucrado(
      id: json['id'] is int ? json['id'] : 0,
      credencial: json['credencial']?.toString() ?? '',
      nombreCompleto: json['nombre_completo']?.toString() ?? '',
      rol: json['rol']?.toString() ?? 'Causante',
      situacionProcesal: json['situacion_procesal']?.toString() ?? 'Sin calificar',
      suspensionPreventiva: json['suspension_preventiva'] == true,
      causaJudicial: json['causa_judicial'] == true,
      escalafon: json['escalafon']?.toString(),
      grado: json['grado']?.toString(),
    );
  }
}
class SuspensionAsociada {
  final int id;
  final String credencial;
  final String nombreCompleto;
  final String? fechaInicio;
  final String estado;
  final String articulo419Incisos;
  final String? fechaFin;
  SuspensionAsociada({
    required this.id,
    required this.credencial,
    required this.nombreCompleto,
    this.fechaInicio,
    required this.estado,
    required this.articulo419Incisos,
    this.fechaFin,
  });
  factory SuspensionAsociada.fromJson(Map<String, dynamic> json) {
    return SuspensionAsociada(
      id: json['id'] is int ? json['id'] : 0,
      credencial: json['credencial']?.toString() ?? '',
      nombreCompleto: json['nombre_completo']?.toString() ?? '',
      fechaInicio: json['fecha_inicio']?.toString(),
      estado: json['estado']?.toString() ?? 'Vigente',
      articulo419Incisos: json['articulo_419_incisos']?.toString() ?? '',
      fechaFin: json['fecha_fin']?.toString(),
    );
  }
}
class NovedadExpediente {
  final int id;
  final String tipoNovedad;
  final String titulo;
  final String descripcion;
  final String usuario;
  final String fecha;
  NovedadExpediente({
    required this.id,
    required this.tipoNovedad,
    required this.titulo,
    required this.descripcion,
    required this.usuario,
    required this.fecha,
  });
  factory NovedadExpediente.fromJson(Map<String, dynamic> json) {
    return NovedadExpediente(
      id: json['id'] is int ? json['id'] : 0,
      tipoNovedad: json['tipo_novedad']?.toString() ?? '',
      titulo: json['titulo']?.toString() ?? '',
      descripcion: json['descripcion']?.toString() ?? '',
      usuario: json['usuario']?.toString() ?? 'Sistema',
      fecha: json['fecha']?.toString() ?? '',
    );
  }
}
class DocumentoExpediente {
  final int id;
  final String tipoDocumento;
  final String nombreOriginal;
  final String nombreArchivo;
  final int version;
  final int tamanoBytes;
  final String fecha;
  DocumentoExpediente({
    required this.id,
    required this.tipoDocumento,
    required this.nombreOriginal,
    required this.nombreArchivo,
    required this.version,
    required this.tamanoBytes,
    required this.fecha,
  });
  factory DocumentoExpediente.fromJson(Map<String, dynamic> json) {
    return DocumentoExpediente(
      id: json['id'] is int ? json['id'] : 0,
      tipoDocumento: json['tipo_documento']?.toString() ?? '',
      nombreOriginal: json['nombre_original']?.toString() ?? '',
      nombreArchivo: json['nombre_archivo']?.toString() ?? '',
      version: json['version'] is int ? json['version'] : 1,
      tamanoBytes: json['tamano_bytes'] is int ? json['tamano_bytes'] : 0,
      fecha: json['fecha']?.toString() ?? '',
    );
  }
}
class VersionExpediente {
  final int id;
  final int numeroVersion;
  final String tipoOperacion;
  final String usuario;
  final String motivo;
  final String fecha;
  VersionExpediente({
    required this.id,
    required this.numeroVersion,
    required this.tipoOperacion,
    required this.usuario,
    required this.motivo,
    required this.fecha,
  });
  factory VersionExpediente.fromJson(Map<String, dynamic> json) {
    return VersionExpediente(
      id: json['id'] is int ? json['id'] : 0,
      numeroVersion: json['numero_version'] is int ? json['numero_version'] : 1,
      tipoOperacion: json['tipo_operacion']?.toString() ?? '',
      usuario: json['usuario']?.toString() ?? '',
      motivo: json['motivo']?.toString() ?? '',
      fecha: json['fecha']?.toString() ?? '',
    );
  }
}