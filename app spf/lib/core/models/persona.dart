class PersonaItem {
  final String credencial;
  final String? nombreApellido;
  final String? apellido;
  final String? nombre;
  final String? escalafon;
  final String? grado;
  final String? origen;
  PersonaItem({
    required this.credencial,
    this.nombreApellido,
    this.apellido,
    this.nombre,
    this.escalafon,
    this.grado,
    this.origen,
  });
  factory PersonaItem.fromJson(Map<String, dynamic> json) {
    return PersonaItem(
      credencial: json['credencial']?.toString() ?? '',
      nombreApellido: json['nombre_apellido']?.toString() ?? json['nombre']?.toString(),
      apellido: json['apellido']?.toString(),
      nombre: json['nombre']?.toString(),
      escalafon: json['escalafon']?.toString(),
      grado: json['grado']?.toString(),
      origen: json['origen_principal']?.toString() ?? json['origen']?.toString(),
    );
  }
  String get displayName {
    if (nombreApellido != null && nombreApellido!.isNotEmpty) return nombreApellido!;
    if (apellido != null && nombre != null) return '$apellido, $nombre';
    return credencial;
  }
}
class PersonaDetalle {
  final PersonaItem persona;
  final List<PersonaExpedienteRelacion> expedientes;
  final List<PersonaSuspensionRelacion> suspensiones;
  PersonaDetalle({
    required this.persona,
    this.expedientes = const [],
    this.suspensiones = const [],
  });
  factory PersonaDetalle.fromJson(Map<String, dynamic> json) {
    final persJson = json['persona'] as Map<String, dynamic>? ?? {};
    final expList = (json['expedientes'] as List? ?? [])
        .map((e) => PersonaExpedienteRelacion.fromJson(e))
        .toList();
    final suspList = (json['suspensiones'] as List? ?? [])
        .map((s) => PersonaSuspensionRelacion.fromJson(s))
        .toList();
    return PersonaDetalle(
      persona: PersonaItem.fromJson(persJson),
      expedientes: expList,
      suspensiones: suspList,
    );
  }
}
class PersonaExpedienteRelacion {
  final int id;
  final String numeroExpediente;
  final String reparticion;
  final String motivo;
  final String estado;
  final String rol;
  final String? fechaInicio;
  PersonaExpedienteRelacion({
    required this.id,
    required this.numeroExpediente,
    required this.reparticion,
    required this.motivo,
    required this.estado,
    required this.rol,
    this.fechaInicio,
  });
  factory PersonaExpedienteRelacion.fromJson(Map<String, dynamic> json) {
    return PersonaExpedienteRelacion(
      id: json['id'] is int ? json['id'] : 0,
      numeroExpediente: json['numero_expediente']?.toString() ?? 'S/N',
      reparticion: json['reparticion']?.toString() ?? 'General',
      motivo: json['motivo']?.toString() ?? '',
      estado: json['estado']?.toString() ?? 'Abierto',
      rol: json['rol']?.toString() ?? 'Causante',
      fechaInicio: json['fecha_inicio']?.toString(),
    );
  }
}
class PersonaSuspensionRelacion {
  final int id;
  final int? expedienteId;
  final String? numeroExpediente;
  final String? fechaInicio;
  final String estado;
  final String articulo419Incisos;
  final String? fechaFin;
  PersonaSuspensionRelacion({
    required this.id,
    this.expedienteId,
    this.numeroExpediente,
    this.fechaInicio,
    required this.estado,
    required this.articulo419Incisos,
    this.fechaFin,
  });
  factory PersonaSuspensionRelacion.fromJson(Map<String, dynamic> json) {
    return PersonaSuspensionRelacion(
      id: json['id'] is int ? json['id'] : 0,
      expedienteId: json['expediente_id'] is int ? json['expediente_id'] : null,
      numeroExpediente: json['numero_expediente']?.toString(),
      fechaInicio: json['fecha_inicio']?.toString(),
      estado: json['estado']?.toString() ?? 'Vigente',
      articulo419Incisos: json['articulo_419_incisos']?.toString() ?? '',
      fechaFin: json['fecha_fin']?.toString(),
    );
  }
}