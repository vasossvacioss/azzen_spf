import 'package:flutter/cupertino.dart';
import 'package:provider/provider.dart';
import '../../../core/config/app_config.dart';
import '../../../core/security/secure_storage_service.dart';
import '../../../core/theme/azzen_colors.dart';
import '../../../core/theme/azzen_typography.dart';
import '../../../shared/widgets/azzen_badge.dart';
import '../../../shared/widgets/azzen_button.dart';
import '../../../shared/widgets/azzen_card.dart';
import '../../../shared/widgets/azzen_divider.dart';
import '../../auth/providers/auth_provider.dart';
class PerfilScreen extends StatefulWidget {
  const PerfilScreen({super.key});
  @override
  State<PerfilScreen> createState() => clase_2001();
}
class clase_2001 extends State<PerfilScreen> {
  String? v_4001;
  @override
  void initState() {
    super.initState();
    m_3001();
  }
  Future<void> m_3001() async {
    final dev = await SecureStorageService.getOrCreateDeviceUuid();
    if (mounted) {
      setState(() {
        v_4001 = dev;
      });
    }
  }
  @override
  Widget build(BuildContext context) {
    final auth = context.watch<AuthProvider>();
    final user = auth.currentUser;
    return CupertinoPageScaffold(
      navigationBar: const CupertinoNavigationBar(
        middle: Text('Perfil y Seguridad'),
      ),
      child: SafeArea(
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            AzzenCard(
              child: Column(
                children: [
                  Container(
                    width: 64,
                    height: 64,
                    decoration: BoxDecoration(
                      color: AzzenColors.navy,
                      shape: BoxShape.circle,
                    ),
                    child: Center(
                      child: Text(
                        (user?.displayName.isNotEmpty == true ? user!.displayName[0] : 'U').toUpperCase(),
                        style: const TextStyle(color: CupertinoColors.white, fontSize: 26, fontWeight: FontWeight.bold),
                      ),
                    ),
                  ),
                  const SizedBox(height: 12),
                  Text(user?.displayName ?? 'Usuario Institucional', style: AzzenTypography.header2),
                  const SizedBox(height: 4),
                  Text('Credencial: ${user?.credential ?? ""}', style: AzzenTypography.captionBold.copyWith(color: AzzenColors.primary)),
                  const SizedBox(height: 8),
                  AzzenBadge.fromStatus(user?.role ?? 'consulta'),
                ],
              ),
            ),
            const SizedBox(height: 12),
            const Text('SEGURIDAD DE LA TERMINAL', style: AzzenTypography.captionBold),
            const SizedBox(height: 8),
            AzzenCard(
              child: Column(
                children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Row(
                        children: [
                          const Icon(CupertinoIcons.viewfinder, size: 20, color: AzzenColors.navy),
                          const SizedBox(width: 10),
                          const Text('Acceso Biométrico', style: AzzenTypography.bodyBold),
                        ],
                      ),
                      CupertinoSwitch(
                        value: auth.isBiometricEnabled,
                        activeTrackColor: AzzenColors.primary,
                        onChanged: auth.isBiometricSupported
                            ? (val) => auth.toggleBiometric(val)
                            : null,
                      ),
                    ],
                  ),
                  const SizedBox(height: 4),
                  Text(
                    auth.isBiometricSupported
                        ? 'Permite desbloquear rápidamente la sesión mediante Face ID o huella dactilar.'
                        : 'Biometría no disponible o no configurada en este dispositivo.',
                    style: AzzenTypography.caption,
                  ),
                  const AzzenDivider(),
                  m_3002('ID Terminal', v_4001 ?? 'Generando...'),
                  const AzzenDivider(),
                  m_3002('Servidor', AppConfig.baseUrl),
                  const AzzenDivider(),
                  m_3002('Versión App', AppConfig.appVersion),
                ],
              ),
            ),
            const SizedBox(height: 12),
            if (user?.permissions.isNotEmpty == true) ...[
              const Text('PERMISOS ASIGNADOS (RBAC)', style: AzzenTypography.captionBold),
              const SizedBox(height: 8),
              AzzenCard(
                child: Wrap(
                  spacing: 6,
                  runSpacing: 6,
                  children: user!.permissions.map((p) {
                    return Container(
                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                      decoration: BoxDecoration(
                        color: AzzenColors.navyLight,
                        borderRadius: BorderRadius.circular(6),
                      ),
                      child: Text(
                        '${p.recurso}: ${p.accion}',
                        style: const TextStyle(fontSize: 11, color: AzzenColors.navy, fontWeight: FontWeight.w600),
                      ),
                    );
                  }).toList(),
                ),
              ),
            ],
            const SizedBox(height: 24),
            AzzenButton(
              label: 'Cerrar Sesión Institucional',
              icon: CupertinoIcons.square_arrow_right,
              variant: AzzenButtonVariant.danger,
              onPressed: () {
                showCupertinoDialog(
                  context: context,
                  builder: (ctx) => CupertinoAlertDialog(
                    title: const Text('Cierre de Sesión'),
                    content: const Text('¿Confirma que desea cerrar su sesión institucional en esta terminal?'),
                    actions: [
                      CupertinoDialogAction(
                        child: const Text('Cancelar'),
                        onPressed: () => Navigator.pop(ctx),
                      ),
                      CupertinoDialogAction(
                        isDestructiveAction: true,
                        child: const Text('Cerrar Sesión'),
                        onPressed: () {
                          Navigator.pop(ctx);
                          auth.logout();
                        },
                      ),
                    ],
                  ),
                );
              },
            ),
          ],
        ),
      ),
    );
  }
  Widget m_3002(String label, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceBetween,
        children: [
          Text(label, style: AzzenTypography.captionBold),
          const SizedBox(width: 8),
          Flexible(
            child: Text(
              value,
              style: AzzenTypography.caption,
              overflow: TextOverflow.ellipsis,
              textAlign: TextAlign.end,
            ),
          ),
        ],
      ),
    );
  }
}