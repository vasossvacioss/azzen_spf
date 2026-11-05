import 'package:flutter/cupertino.dart';
import '../../../core/models/user_session.dart';
import '../../../core/networking/api_client.dart';
import '../../../core/networking/api_exceptions.dart';
import '../../../core/security/biometric_service.dart';
import '../../../core/security/secure_storage_service.dart';
enum AuthStatus {
  unauthenticated,
  authenticating,
  totpRequired,
  devicePending,
  authenticated,
}
class AuthProvider extends ChangeNotifier {
  AuthStatus v_4008 = AuthStatus.unauthenticated;
  AuthStatus get status => v_4008;
  UserSession? v_4001;
  UserSession? get currentUser => v_4001;
  String? v_4002;
  String? get errorMessage => v_4002;
  String? v_4003;
  String? get pendingChallengeId => v_4003;
  String? v_4004;
  String? get pendingCredential => v_4004;
  String? v_4005;
  String? get pendingDisplayName => v_4005;
  bool v_4006 = false;
  bool get isBiometricSupported => v_4006;
  bool v_4007 = false;
  bool get isBiometricEnabled => v_4007;
  AuthProvider() {
    m_3001();
  }
  Future<void> m_3001() async {
    v_4006 = await BiometricService.isBiometricAvailable();
    v_4007 = await SecureStorageService.isBiometricEnabled();
    final token = await SecureStorageService.getToken();
    if (token != null && token.isNotEmpty) {
      await loadCurrentUser();
      return;
    }
    final pending = await SecureStorageService.getPendingChallenge();
    if (pending != null) {
      v_4003 = pending['challengeId'];
      v_4004 = pending['credential'];
      v_4005 = pending['displayName'];
      v_4008 = AuthStatus.totpRequired;
      notifyListeners();
    }
  }
  Future<bool> login({
    required String credential,
    required String password,
  }) async {
    v_4008 = AuthStatus.authenticating;
    v_4002 = null;
    notifyListeners();
    try {
      final deviceUuid = await SecureStorageService.getOrCreateDeviceUuid();
      final response = await ApiClient.post('/api/mobile/auth/login', body: {
        'credential': credential.trim(),
        'password': password.trim(),
        'dispositivo_uuid': deviceUuid,
        'dispositivo_nombre': 'Dispositivo Móvil Institucional',
        'dispositivo_os': 'Mobile Client',
      });
      final statusStr = response['status']?.toString();
      if (statusStr == 'totp_required') {
        v_4008 = AuthStatus.totpRequired;
        v_4003 = response['challenge_id']?.toString();
        v_4004 = response['credential']?.toString();
        v_4005 = response['display_name']?.toString();
        if (v_4003 != null && v_4004 != null) {
          await SecureStorageService.savePendingChallenge(
            challengeId: v_4003!,
            credential: v_4004!,
            displayName: v_4005,
          );
        }
        notifyListeners();
        return false;
      } else if (statusStr == 'device_pending') {
        v_4008 = AuthStatus.devicePending;
        v_4002 = response['message']?.toString() ?? 'Dispositivo pendiente de autorización institucional.';
        notifyListeners();
        return false;
      } else if (statusStr == 'authenticated') {
        final token = response['token']?.toString();
        if (token != null) {
          await SecureStorageService.saveToken(token);
          final userJson = response['user'] as Map<String, dynamic>? ?? {};
          await SecureStorageService.saveUserData(
            credential: userJson['credential']?.toString() ?? credential,
            displayName: userJson['display_name']?.toString() ?? 'Agente',
            role: userJson['role']?.toString() ?? 'consulta',
          );
          await loadCurrentUser();
          return true;
        }
      }
      v_4008 = AuthStatus.unauthenticated;
      v_4002 = 'Respuesta inesperada del servidor de autenticación.';
      notifyListeners();
      return false;
    } catch (e) {
      v_4008 = AuthStatus.unauthenticated;
      v_4002 = e.toString();
      notifyListeners();
      return false;
    }
  }
  Future<bool> verifyTotp(String code) async {
    if (v_4003 == null) {
      v_4002 = 'Desafío no encontrado.';
      notifyListeners();
      return false;
    }
    v_4008 = AuthStatus.authenticating;
    v_4002 = null;
    notifyListeners();
    try {
      final response = await ApiClient.post('/api/mobile/auth/totp', body: {
        'challenge_id': v_4003,
        'code': code.trim(),
      });
      if (response['status'] == 'authenticated') {
        await SecureStorageService.clearPendingChallenge();
        v_4003 = null;
        v_4004 = null;
        v_4005 = null;
        final token = response['token']?.toString();
        if (token != null) {
          await SecureStorageService.saveToken(token);
          final userJson = response['user'] as Map<String, dynamic>? ?? {};
          await SecureStorageService.saveUserData(
            credential: userJson['credential']?.toString() ?? '',
            displayName: userJson['display_name']?.toString() ?? 'Agente',
            role: userJson['role']?.toString() ?? 'consulta',
          );
          await loadCurrentUser();
          return true;
        }
      }
      v_4008 = AuthStatus.totpRequired;
      v_4002 = 'Código incorrecto o expirado.';
      notifyListeners();
      return false;
    } catch (e) {
      v_4008 = AuthStatus.totpRequired;
      v_4002 = e.toString();
      notifyListeners();
      return false;
    }
  }
  Future<bool> unlockWithBiometrics() async {
    if (!v_4006 || !v_4007) return false;
    final token = await SecureStorageService.getToken();
    if (token == null || token.isEmpty) return false;
    final authenticated = await BiometricService.authenticate();
    if (authenticated) {
      return await loadCurrentUser();
    }
    return false;
  }
  Future<void> toggleBiometric(bool enable) async {
    if (enable) {
      final authenticated = await BiometricService.authenticate(
        reason: 'Habilitar acceso biométrico para AZZEN SPF Mobile',
      );
      if (authenticated) {
        await SecureStorageService.setBiometricEnabled(true);
        v_4007 = true;
        notifyListeners();
      }
    } else {
      await SecureStorageService.setBiometricEnabled(false);
      v_4007 = false;
      notifyListeners();
    }
  }
  Future<bool> loadCurrentUser() async {
    try {
      final response = await ApiClient.get('/api/mobile/auth/me');
      v_4001 = UserSession.fromJson(response);
      v_4008 = AuthStatus.authenticated;
      v_4002 = null;
      notifyListeners();
      return true;
    } on UnauthorizedException {
      await logout();
      return false;
    } catch (e) {
      final savedCred = await SecureStorageService.getSavedCredential();
      final savedName = await SecureStorageService.getSavedDisplayName();
      final savedRole = await SecureStorageService.getSavedRole();
      if (savedCred != null && savedName != null) {
        v_4001 = UserSession(
          id: 0,
          credential: savedCred,
          displayName: savedName,
          role: savedRole ?? 'consulta',
        );
        v_4008 = AuthStatus.authenticated;
        notifyListeners();
        return true;
      }
      v_4008 = AuthStatus.unauthenticated;
      notifyListeners();
      return false;
    }
  }
  Future<void> cancelTotp() async {
    await SecureStorageService.clearPendingChallenge();
    v_4003 = null;
    v_4004 = null;
    v_4005 = null;
    v_4008 = AuthStatus.unauthenticated;
    v_4002 = null;
    notifyListeners();
  }
  Future<void> logout() async {
    try {
      await ApiClient.post('/api/mobile/auth/logout');
    } catch (_) {}
    await SecureStorageService.deleteToken();
    await SecureStorageService.clearPendingChallenge();
    v_4001 = null;
    v_4008 = AuthStatus.unauthenticated;
    v_4003 = null;
    v_4004 = null;
    v_4005 = null;
    notifyListeners();
  }
  void clearError() {
    v_4002 = null;
    notifyListeners();
  }
}