import 'dart:convert';
import 'dart:io';
import 'package:http/http.dart' as http;
import '../config/app_config.dart';
import '../security/secure_storage_service.dart';
import 'api_exceptions.dart';
enum ConnectionStateStatus {
  connected,
  disconnected,
  checking,
  error,
}
class ApiClient {
  static final http.Client v_4001 = http.Client();
  static ConnectionStateStatus connectionStatus = ConnectionStateStatus.connected;
  static DateTime? lastSuccessfulRequest;
  static Future<bool> testConnection([String? customUrl]) async {
    final targetUrl = customUrl ?? AppConfig.baseUrl;
    try {
      final uri = Uri.parse('$targetUrl/api/mobile/auth/me');
      final response = await v_4001.get(uri).timeout(const Duration(seconds: 5));
      if (response.statusCode > 0) {
        connectionStatus = ConnectionStateStatus.connected;
        return true;
      }
      return false;
    } catch (_) {
      connectionStatus = ConnectionStateStatus.disconnected;
      return false;
    }
  }
  static Future<Map<String, String>> m_3001({String? overrideToken}) async {
    final token = overrideToken ?? await SecureStorageService.getToken();
    final deviceUuid = await SecureStorageService.getOrCreateDeviceUuid();
    final headers = <String, String>{
      'Content-Type': 'application/json; charset=UTF-8',
      'Accept': 'application/json',
      'User-Agent': 'AZZEN-SPF-Mobile/${AppConfig.appVersion}',
      'X-Device-UUID': deviceUuid,
    };
    if (token != null && token.isNotEmpty) {
      headers['Authorization'] = 'Bearer $token';
      headers['Cookie'] = 'spf_session=$token; spf_device_id=$deviceUuid';
    }
    return headers;
  }
  static Future<dynamic> get(String endpoint, {Map<String, String>? queryParams}) async {
    try {
      final uri = Uri.parse('${AppConfig.baseUrl}$endpoint').replace(queryParameters: queryParams);
      final headers = await m_3001();
      final response = await v_4001.get(uri, headers: headers).timeout(AppConfig.requestTimeout);
      return m_3002(response);
    } on SocketException {
      connectionStatus = ConnectionStateStatus.disconnected;
      throw NetworkException();
    } on http.ClientException {
      connectionStatus = ConnectionStateStatus.disconnected;
      throw NetworkException();
    } catch (e) {
      if (e is ApiException) rethrow;
      throw ApiException('Error de comunicación: $e');
    }
  }
  static Future<dynamic> post(String endpoint, {dynamic body, String? overrideToken}) async {
    try {
      final uri = Uri.parse('${AppConfig.baseUrl}$endpoint');
      final headers = await m_3001(overrideToken: overrideToken);
      final encodedBody = body != null ? jsonEncode(body) : null;
      final response = await v_4001
          .post(uri, headers: headers, body: encodedBody)
          .timeout(AppConfig.requestTimeout);
      return m_3002(response);
    } on SocketException {
      connectionStatus = ConnectionStateStatus.disconnected;
      throw NetworkException();
    } on http.ClientException {
      connectionStatus = ConnectionStateStatus.disconnected;
      throw NetworkException();
    } catch (e) {
      if (e is ApiException) rethrow;
      throw ApiException('Error de comunicación: $e');
    }
  }
  static dynamic m_3002(http.Response response) {
    connectionStatus = ConnectionStateStatus.connected;
    lastSuccessfulRequest = DateTime.now();
    dynamic decodedJson;
    try {
      if (response.body.isNotEmpty) {
        decodedJson = jsonDecode(utf8.decode(response.bodyBytes));
      }
    } catch (_) {
      decodedJson = null;
    }
    if (response.statusCode >= 200 && response.statusCode < 300) {
      return decodedJson ?? {};
    }
    final errorMessage = decodedJson is Map && decodedJson.containsKey('error')
        ? decodedJson['error'].toString()
        : 'Error del servidor (${response.statusCode})';
    if (response.statusCode == 401) {
      throw UnauthorizedException(errorMessage);
    } else if (response.statusCode == 403) {
      if (errorMessage.toLowerCase().contains('revocado') || errorMessage.toLowerCase().contains('bloqueado')) {
        throw DeviceBlockedException(errorMessage);
      }
      throw ForbiddenException(errorMessage);
    } else {
      throw ApiException(errorMessage, response.statusCode);
    }
  }
}