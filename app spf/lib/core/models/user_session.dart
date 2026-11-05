class UserSession {
  final int id;
  final String credential;
  final String displayName;
  final String role;
  final String? deviceUuid;
  final List<PermissionItem> permissions;
  UserSession({
    required this.id,
    required this.credential,
    required this.displayName,
    required this.role,
    this.deviceUuid,
    this.permissions = const [],
  });
  factory UserSession.fromJson(Map<String, dynamic> json) {
    final permsRaw = json['permisos'] as List? ?? [];
    return UserSession(
      id: json['id'] is int ? json['id'] : (json['user_id'] is int ? json['user_id'] : 0),
      credential: json['credential']?.toString() ?? '',
      displayName: json['display_name']?.toString() ?? 'Agente',
      role: json['role']?.toString() ?? 'consulta',
      deviceUuid: json['dispositivo_uuid']?.toString(),
      permissions: permsRaw.map((p) => PermissionItem.fromJson(p)).toList(),
    );
  }
  bool hasPermission(String recurso, String accion) {
    if (role == 'superusuario' || role == 'admin_tecnico') return true;
    return permissions.any((p) => p.recurso == recurso && p.accion == accion);
  }
}
class PermissionItem {
  final String recurso;
  final String accion;
  PermissionItem({required this.recurso, required this.accion});
  factory PermissionItem.fromJson(Map<String, dynamic> json) {
    return PermissionItem(
      recurso: json['recurso']?.toString() ?? '',
      accion: json['accion']?.toString() ?? '',
    );
  }
}