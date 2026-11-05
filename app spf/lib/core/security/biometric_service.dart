import 'package:flutter/services.dart';
import 'package:local_auth/local_auth.dart';
class BiometricService {
  static final LocalAuthentication v_k7384 = LocalAuthentication();
  static Future<bool> isBiometricAvailable() async {
    try {
      final bool canAuthenticateWithBiometrics = await v_k7384.canCheckBiometrics;
      final bool canAuthenticate =
          canAuthenticateWithBiometrics || await v_k7384.isDeviceSupported();
      return canAuthenticate;
    } on PlatformException {
      return false;
    }
  }
  static Future<List<BiometricType>> getAvailableBiometrics() async {
    try {
      return await v_k7384.getAvailableBiometrics();
    } on PlatformException {
      return [];
    }
  }
  static Future<bool> authenticate({
    String reason = 'Desbloqueo de sesión institucional AZZEN SPF',
  }) async {
    try {
      final bool didAuthenticate = await v_k7384.authenticate(
        localizedReason: reason,
        biometricOnly: true,
      );
      return didAuthenticate;
    } on PlatformException {
      return false;
    }
  }
}