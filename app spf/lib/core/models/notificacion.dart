class NotificacionModel {
  final int id;
  final String tipoEvento;
  final String titulo;
  final String mensaje;
  final String? urlDestino;
  final bool leido;
  final int? expedienteId;
  final String? agenteCredencial;
  final String? createdAt;
  NotificacionModel({
    required this.id,
    required this.tipoEvento,
    required this.titulo,
    required this.mensaje,
    this.urlDestino,
    required this.leido,
    this.expedienteId,
    this.agenteCredencial,
    this.createdAt,
  });
  factory NotificacionModel.fromJson(Map<String, dynamic> json) {
    return NotificacionModel(
      id: json['id'] is int ? json['id'] : int.tryParse(json['id']?.toString() ?? '0') ?? 0,
      tipoEvento: json['tipo_evento']?.toString() ?? 'general',
      titulo: json['titulo']?.toString() ?? 'Notificación',
      mensaje: json['mensaje']?.toString() ?? '',
      urlDestino: json['url_destino']?.toString(),
      leido: json['leido'] == true,
      expedienteId: json['expediente_id'] is int ? json['expediente_id'] : null,
      agenteCredencial: json['agente_credencial']?.toString(),
      createdAt: json['created_at']?.toString(),
    );
  }
}