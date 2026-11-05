class SuspensionItem {
  final int id;
  final int? expedienteId;
  final String? numeroExpediente;
  final String? reparticion;
  final String credencial;
  final String nombreCompleto;
  final String? fechaInicio;
  final String articulo419Incisos;
  final String estado;
  final String? fechaFin;
  final String? motivoFin;
  final String? fundamentoFin;
  final String? archivoDisposicionPdf;
  final String? archivoLevantamientoPdf;
  final String? observaciones;
  SuspensionItem({
    required this.id,
    this.expedienteId,
    this.numeroExpediente,
    this.reparticion,
    required this.credencial,
    required this.nombreCompleto,
    this.fechaInicio,
    required this.articulo419Incisos,
    required this.estado,
    this.fechaFin,
    this.motivoFin,
    this.fundamentoFin,
    this.archivoDisposicionPdf,
    this.archivoLevantamientoPdf,
    this.observaciones,
  });
  factory SuspensionItem.fromJson(Map<String, dynamic> json) {
    return SuspensionItem(
      id: json['id'] is int ? json['id'] : int.tryParse(json['id']?.toString() ?? '0') ?? 0,
      expedienteId: json['expediente_id'] is int ? json['expediente_id'] : null,
      numeroExpediente: json['numero_expediente']?.toString(),
      reparticion: json['reparticion']?.toString(),
      credencial: json['credencial']?.toString() ?? '',
      nombreCompleto: json['nombre_completo']?.toString() ?? 'Sin nombre',
      fechaInicio: json['fecha_inicio']?.toString(),
      articulo419Incisos: json['articulo_419_incisos']?.toString() ?? '',
      estado: json['estado']?.toString() ?? 'Vigente',
      fechaFin: json['fecha_fin']?.toString(),
      motivoFin: json['motivo_fin']?.toString(),
      fundamentoFin: json['fundamento_fin']?.toString(),
      archivoDisposicionPdf: json['archivo_disposicion_pdf']?.toString(),
      archivoLevantamientoPdf: json['archivo_levantamiento_pdf']?.toString(),
      observaciones: json['observaciones']?.toString(),
    );
  }
}