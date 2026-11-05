class GlobalSearchResult {
  final List<SearchExpedienteResult> expedientes;
  final List<SearchAgenteResult> agentes;
  final List<SearchSuspensionResult> suspensiones;
  GlobalSearchResult({
    this.expedientes = const [],
    this.agentes = const [],
    this.suspensiones = const [],
  });
  factory GlobalSearchResult.fromJson(Map<String, dynamic> json) {
    final expList = (json['expedientes'] as List? ?? [])
        .map((e) => SearchExpedienteResult.fromJson(e))
        .toList();
    final agList = (json['agentes'] as List? ?? [])
        .map((a) => SearchAgenteResult.fromJson(a))
        .toList();
    final suspList = (json['suspensiones'] as List? ?? [])
        .map((s) => SearchSuspensionResult.fromJson(s))
        .toList();
    return GlobalSearchResult(
      expedientes: expList,
      agentes: agList,
      suspensiones: suspList,
    );
  }
  bool get isEmpty => expedientes.isEmpty && agentes.isEmpty && suspensiones.isEmpty;
  int get totalCount => expedientes.length + agentes.length + suspensiones.length;
}
class SearchExpedienteResult {
  final int id;
  final String numero;
  final String caratula;
  final String estado;
  SearchExpedienteResult({
    required this.id,
    required this.numero,
    required this.caratula,
    required this.estado,
  });
  factory SearchExpedienteResult.fromJson(Map<String, dynamic> json) {
    return SearchExpedienteResult(
      id: json['id'] is int ? json['id'] : int.tryParse(json['id']?.toString() ?? '0') ?? 0,
      numero: json['numero']?.toString() ?? '',
      caratula: json['caratula']?.toString() ?? '',
      estado: json['estado']?.toString() ?? '',
    );
  }
}
class SearchAgenteResult {
  final String credencial;
  final String nombre;
  final String origen;
  SearchAgenteResult({
    required this.credencial,
    required this.nombre,
    required this.origen,
  });
  factory SearchAgenteResult.fromJson(Map<String, dynamic> json) {
    return SearchAgenteResult(
      credencial: json['credencial']?.toString() ?? '',
      nombre: json['nombre']?.toString() ?? '',
      origen: json['origen']?.toString() ?? '',
    );
  }
}
class SearchSuspensionResult {
  final int id;
  final int? expedienteId;
  final String credencial;
  final String nombre;
  final String estado;
  final String articulo;
  SearchSuspensionResult({
    required this.id,
    this.expedienteId,
    required this.credencial,
    required this.nombre,
    required this.estado,
    required this.articulo,
  });
  factory SearchSuspensionResult.fromJson(Map<String, dynamic> json) {
    return SearchSuspensionResult(
      id: json['id'] is int ? json['id'] : int.tryParse(json['id']?.toString() ?? '0') ?? 0,
      expedienteId: json['expediente_id'] is int ? json['expediente_id'] : null,
      credencial: json['credencial']?.toString() ?? '',
      nombre: json['nombre']?.toString() ?? '',
      estado: json['estado']?.toString() ?? '',
      articulo: json['articulo']?.toString() ?? '',
    );
  }
}