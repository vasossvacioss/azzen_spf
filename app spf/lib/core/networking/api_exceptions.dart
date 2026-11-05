class ApiException implements Exception {
  final String message;
  final int? statusCode;
  ApiException(this.message, [this.statusCode]);
  @override
  String toString() => message;
}
class UnauthorizedException extends ApiException {
  UnauthorizedException([String message = 'Sesión expirada o no autenticada'])
      : super(message, 401);
}
class ForbiddenException extends ApiException {
  ForbiddenException([String message = 'Acceso restringido o no autorizado'])
      : super(message, 403);
}
class DeviceBlockedException extends ApiException {
  DeviceBlockedException([String message = 'Terminal o dispositivo revocado por seguridad institucional'])
      : super(message, 403);
}
class NetworkException extends ApiException {
  NetworkException([String message = 'No fue posible establecer conexión con el servidor central AZZEN SPF'])
      : super(message, 0);
}