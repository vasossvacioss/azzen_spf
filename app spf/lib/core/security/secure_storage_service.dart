import 'package:flutter_secure_storage/flutter_secure_storage.dart';
import 'dart:math';
class SecureStorageService {
  static const v_4001 = FlutterSecureStorage();
  static const String v_4002 = 'azzen_token';
  static const String v_4003 = 'azzen_device_uuid';
  static const String v_4004 = 'azzen_credential';
  static const String v_4005 = 'azzen_display_name';
  static const String v_4006 = 'azzen_user_role';
  static const String v_4007 = 'azzen_biometric_enabled';
  static const String v_4008 = 'azzen_pending_challenge_id';
  static const String v_4009 = 'azzen_pending_credential';
  static const String v_4010 = 'azzen_pending_display_name';
  static const String v_4011 = 'azzen_pending_timestamp';
  static Future<void> savePendingChallenge({
    required String challengeId,
    required String credential,
    String? displayName,
  }) async {
    await v_4001.write(key: v_4008, value: challengeId);
    await v_4001.write(key: v_4009, value: credential);
    if (displayName != null) {
      await v_4001.write(key: v_4010, value: displayName);
    }
    await v_4001.write(
      key: v_4011,
      value: DateTime.now().millisecondsSinceEpoch.toString(),
    );
  }
  static Future<Map<String, String>?> getPendingChallenge() async {
    final challengeId = await v_4001.read(key: v_4008);
    final credential = await v_4001.read(key: v_4009);
    final stampStr = await v_4001.read(key: v_4011);
    if (challengeId == null || credential == null || stampStr == null) {
      return null;
    }
    final stamp = int.tryParse(stampStr) ?? 0;
    final ageMs = DateTime.now().millisecondsSinceEpoch - stamp;
    if (ageMs > 300000) {
      await clearPendingChallenge();
      return null;
    }
    final displayName = await v_4001.read(key: v_4010);
    return {
      'challengeId': challengeId,
      'credential': credential,
      'displayName': displayName ?? '',
    };
  }
  static Future<void> clearPendingChallenge() async {
    await v_4001.delete(key: v_4008);
    await v_4001.delete(key: v_4009);
    await v_4001.delete(key: v_4010);
    await v_4001.delete(key: v_4011);
  }
  static Future<void> saveToken(String token) async {
    await v_4001.write(key: v_4002, value: token);
  }
  static Future<String?> getToken() async {
    return await v_4001.read(key: v_4002);
  }
  static Future<void> deleteToken() async {
    await v_4001.delete(key: v_4002);
  }
  static Future<String> getOrCreateDeviceUuid() async {
    String? uuid = await v_4001.read(key: v_4003);
    if (uuid == null || uuid.isEmpty) {
      final rand = Random();
      final stamp = DateTime.now().millisecondsSinceEpoch;
      final part1 = rand.nextInt(0xFFFFFF).toRadixString(16).padLeft(6, '0');
      final part2 = rand.nextInt(0xFFFFFF).toRadixString(16).padLeft(6, '0');
      uuid = 'dev-spf-$part1-$part2-$stamp';
      await v_4001.write(key: v_4003, value: uuid);
    }
    return uuid;
  }
  static Future<void> saveUserData({
    required String credential,
    required String displayName,
    required String role,
  }) async {
    await v_4001.write(key: v_4004, value: credential);
    await v_4001.write(key: v_4005, value: displayName);
    await v_4001.write(key: v_4006, value: role);
  }
  static Future<String?> getSavedCredential() async {
    return await v_4001.read(key: v_4004);
  }
  static Future<String?> getSavedDisplayName() async {
    return await v_4001.read(key: v_4005);
  }
  static Future<String?> getSavedRole() async {
    return await v_4001.read(key: v_4006);
  }
  static Future<void> setBiometricEnabled(bool enabled) async {
    await v_4001.write(key: v_4007, value: enabled ? '1' : '0');
  }
  static Future<bool> isBiometricEnabled() async {
    final val = await v_4001.read(key: v_4007);
    return val == '1';
  }
  static Future<void> clearAll() async {
    await v_4001.delete(key: v_4002);
    await v_4001.delete(key: v_4005);
    await v_4001.delete(key: v_4006);
  }
}