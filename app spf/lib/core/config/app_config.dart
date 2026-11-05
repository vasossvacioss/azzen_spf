import 'dart:io';
import 'package:shared_preferences/shared_preferences.dart';
class AppConfig {
  static const String appName = 'AZZEN SPF Mobile';
  static const String appVersion = '1.0.0';
  static const String v_4001 = 'azzen_server_url';
  static const String lanServerUrl = 'http:
  static const String emulatorServerUrl = 'http:
  static const String loopbackServerUrl = 'http:
  static String get defaultBaseUrl {
    if (Platform.isAndroid) {
      return lanServerUrl;
    } else {
      return loopbackServerUrl;
    }
  }
  static String baseUrl = lanServerUrl;
  static const Duration requestTimeout = Duration(seconds: 15);
  static const Duration sessionInactivityTimeout = Duration(minutes: 30);
  static Future<void> init() async {
    try {
      final prefs = await SharedPreferences.getInstance();
      final savedUrl = prefs.getString(v_4001);
      if (savedUrl != null && savedUrl.trim().isNotEmpty) {
        baseUrl = savedUrl.trim();
      } else {
        baseUrl = defaultBaseUrl;
      }
    } catch (_) {
      baseUrl = defaultBaseUrl;
    }
  }
  static Future<void> setBaseUrl(String newUrl) async {
    var sanitized = newUrl.trim();
    if (sanitized.endsWith('/')) {
      sanitized = sanitized.substring(0, sanitized.length - 1);
    }
    baseUrl = sanitized;
    try {
      final prefs = await SharedPreferences.getInstance();
      await prefs.setString(v_4001, sanitized);
    } catch (_) {}
  }
}