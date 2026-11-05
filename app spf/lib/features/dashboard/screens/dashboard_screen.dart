import 'package:flutter/cupertino.dart';
import 'package:provider/provider.dart';
import '../../../core/networking/api_client.dart';
import '../../../core/theme/azzen_colors.dart';
import '../../../core/theme/azzen_typography.dart';
import '../../../shared/widgets/azzen_card.dart';
import '../../../shared/widgets/connection_status_bar.dart';
import '../../auth/providers/auth_provider.dart';
import '../../expedientes/screens/expedientes_list_screen.dart';
import '../../global_search/screens/global_search_screen.dart';
import '../../notificaciones/screens/notificaciones_screen.dart';
import '../../personas/screens/personas_list_screen.dart';
import '../../perfil/screens/perfil_screen.dart';
import '../../suspensiones/screens/suspensiones_list_screen.dart';
class DashboardScreen extends StatefulWidget {
  const DashboardScreen({super.key});
  @override
  State<DashboardScreen> createState() => clase_2001();
}
class clase_2001 extends State<DashboardScreen> {
  int v_4001 = 200;
  int v_4002 = 222;
  int v_4003 = 100;
  int v_4004 = 0;
  @override
  void initState() {
    super.initState();
    m_3001();
  }
  Future<void> m_3001() async {
    try {
      final data = await ApiClient.get('/modulos/dashboard', queryParams: {'format': 'json'});
      final stats = data['estadisticas'] as Map<String, dynamic>? ?? {};
      final notifsData = await ApiClient.get('/api/notificaciones');
      final notifsList = notifsData['notificaciones'] as List? ?? (notifsData is List ? notifsData : []);
      final unread = notifsList.where((n) => n['leido'] == false).length;
      if (mounted) {
        setState(() {
          v_4001 = stats['expedientes'] is int ? stats['expedientes'] : v_4001;
          v_4002 = stats['involucrados'] is int ? stats['involucrados'] : v_4002;
          v_4003 = stats['suspensiones_activas'] is int ? stats['suspensiones_activas'] : v_4003;
          v_4004 = unread;
        });
      }
    } catch (_) {}
  }
  @override
  Widget build(BuildContext context) {
    final auth = context.watch<AuthProvider>();
    final user = auth.currentUser;
    return CupertinoPageScaffold(
      backgroundColor: AzzenColors.background,
      navigationBar: CupertinoNavigationBar(
        backgroundColor: AzzenColors.surfaceSecondary,
        leading: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            ClipRRect(
              borderRadius: BorderRadius.circular(6),
              child: Image.asset(
                'assets/images/azzen_spf_app.png',
                width: 24,
                height: 24,
                fit: BoxFit.contain,
              ),
            ),
            const SizedBox(width: 8),
            const Text('AZZEN SPF', style: TextStyle(fontWeight: FontWeight.w800, letterSpacing: 0.5)),
          ],
        ),
        trailing: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            CupertinoButton(
              padding: EdgeInsets.zero,
              child: Stack(
                children: [
                  const Icon(CupertinoIcons.bell, size: 22, color: AzzenColors.navy),
                  if (v_4004 > 0)
                    Positioned(
                      right: 0,
                      top: 0,
                      child: Container(
                        padding: const EdgeInsets.all(2),
                        decoration: const BoxDecoration(
                          color: AzzenColors.primary,
                          shape: BoxShape.circle,
                        ),
                        constraints: const BoxConstraints(minWidth: 14, minHeight: 14),
                        child: Text(
                          '$v_4004',
                          style: const TextStyle(color: CupertinoColors.white, fontSize: 9, fontWeight: FontWeight.bold),
                          textAlign: TextAlign.center,
                        ),
                      ),
                    ),
                ],
              ),
              onPressed: () async {
                await Navigator.push(
                  context,
                  CupertinoPageRoute(builder: (_) => const NotificacionesScreen()),
                );
                m_3001();
              },
            ),
            const SizedBox(width: 8),
            CupertinoButton(
              padding: EdgeInsets.zero,
              child: const Icon(CupertinoIcons.person_crop_circle, size: 24, color: AzzenColors.navy),
              onPressed: () {
                Navigator.push(
                  context,
                  CupertinoPageRoute(builder: (_) => const PerfilScreen()),
                );
              },
            ),
          ],
        ),
      ),
      child: SafeArea(
        child: Column(
          children: [
            const ConnectionStatusBar(),
            Expanded(
              child: ListView(
                padding: const EdgeInsets.all(16),
                children: [
                  AzzenCard(
                    backgroundColor: AzzenColors.navy,
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          'Bienvenido, ${user?.displayName ?? "Agente"}',
                          style: AzzenTypography.header3.copyWith(color: CupertinoColors.white),
                        ),
                        const SizedBox(height: 4),
                        Text(
                          'Credencial ${user?.credential ?? ""} · Rol: ${(user?.role ?? "Consulta").toUpperCase()}',
                          style: AzzenTypography.caption.copyWith(color: AzzenColors.textMuted),
                        ),
                      ],
                    ),
                  ),
                  GestureDetector(
                    onTap: () {
                      Navigator.push(
                        context,
                        CupertinoPageRoute(builder: (_) => const GlobalSearchScreen()),
                      );
                    },
                    child: Container(
                      margin: const EdgeInsets.only(bottom: 16),
                      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
                      decoration: BoxDecoration(
                        color: AzzenColors.surfaceSecondary,
                        borderRadius: BorderRadius.circular(12),
                        border: Border.all(color: AzzenColors.border),
                      ),
                      child: Row(
                        children: [
                          const Icon(CupertinoIcons.search, color: AzzenColors.primary, size: 20),
                          const SizedBox(width: 10),
                          Text(
                            'Búsqueda transversal (personas, expedientes...)',
                            style: AzzenTypography.body.copyWith(color: AzzenColors.textMuted),
                          ),
                        ],
                      ),
                    ),
                  ),
                  Row(
                    children: [
                      Expanded(
                        child: m_3002(
                          title: 'Expedientes',
                          value: '$v_4001',
                          icon: CupertinoIcons.doc_text_fill,
                          color: AzzenColors.primary,
                          onTap: () => Navigator.push(
                            context,
                            CupertinoPageRoute(builder: (_) => const ExpedientesListScreen()),
                          ),
                        ),
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: m_3002(
                          title: 'Involucrados',
                          value: '$v_4002',
                          icon: CupertinoIcons.person_2_fill,
                          color: AzzenColors.navy,
                          onTap: () => Navigator.push(
                            context,
                            CupertinoPageRoute(builder: (_) => const PersonasListScreen()),
                          ),
                        ),
                      ),
                      const SizedBox(width: 10),
                      Expanded(
                        child: m_3002(
                          title: 'Susp. Activas',
                          value: '$v_4003',
                          icon: CupertinoIcons.shield_fill,
                          color: AzzenColors.danger,
                          onTap: () => Navigator.push(
                            context,
                            CupertinoPageRoute(
                              builder: (_) => const SuspensionesListScreen(initialFilterEstado: 'Vigente'),
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 20),
                  const Text('MÓDULOS DE CONSULTA Y GESTIÓN', style: AzzenTypography.captionBold),
                  const SizedBox(height: 10),
                  m_3003(
                    title: 'Expedientes y Sumarios',
                    subtitle: 'Consulta de actuaciones, carátulas, instructores y novedades',
                    icon: CupertinoIcons.folder_fill,
                    iconBg: AzzenColors.primaryLight,
                    iconColor: AzzenColors.primary,
                    onTap: () => Navigator.push(
                      context,
                      CupertinoPageRoute(builder: (_) => const ExpedientesListScreen()),
                    ),
                  ),
                  m_3003(
                    title: 'Personas y Agentes',
                    subtitle: 'Búsqueda por credencial, grado, escalafón y sumarios relacionados',
                    icon: CupertinoIcons.person_3_fill,
                    iconBg: AzzenColors.navyLight,
                    iconColor: AzzenColors.navy,
                    onTap: () => Navigator.push(
                      context,
                      CupertinoPageRoute(builder: (_) => const PersonasListScreen()),
                    ),
                  ),
                  m_3003(
                    title: 'Suspensiones Preventivas',
                    subtitle: 'Verificación de Art. 419, incisos imputados y fechas de cese',
                    icon: CupertinoIcons.exclamationmark_shield_fill,
                    iconBg: AzzenColors.dangerBackground,
                    iconColor: AzzenColors.danger,
                    onTap: () => Navigator.push(
                      context,
                      CupertinoPageRoute(builder: (_) => const SuspensionesListScreen()),
                    ),
                  ),
                  m_3003(
                    title: 'Búsqueda Transversal',
                    subtitle: 'Localización inmediata de personas, causas y medidas cautelares',
                    icon: CupertinoIcons.search_circle_fill,
                    iconBg: AzzenColors.infoBackground,
                    iconColor: AzzenColors.info,
                    onTap: () => Navigator.push(
                      context,
                      CupertinoPageRoute(builder: (_) => const GlobalSearchScreen()),
                    ),
                  ),
                  m_3003(
                    title: 'Centro de Notificaciones',
                    subtitle: 'Novedades oficiales, asignaciones de instructores y avisos',
                    icon: CupertinoIcons.bell_fill,
                    iconBg: AzzenColors.warningBackground,
                    iconColor: AzzenColors.warning,
                    onTap: () => Navigator.push(
                      context,
                      CupertinoPageRoute(builder: (_) => const NotificacionesScreen()),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
  Widget m_3002({
    required String title,
    required String value,
    required IconData icon,
    required Color color,
    required VoidCallback onTap,
  }) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          color: AzzenColors.surfaceCard,
          borderRadius: BorderRadius.circular(10),
          border: Border.all(color: AzzenColors.border),
        ),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Icon(icon, size: 20, color: color),
            const SizedBox(height: 8),
            Text(value, style: AzzenTypography.header2),
            const SizedBox(height: 2),
            Text(title, style: AzzenTypography.caption, maxLines: 1, overflow: TextOverflow.ellipsis),
          ],
        ),
      ),
    );
  }
  Widget m_3003({
    required String title,
    required String subtitle,
    required IconData icon,
    required Color iconBg,
    required Color iconColor,
    required VoidCallback onTap,
  }) {
    return AzzenCard(
      onTap: onTap,
      child: Row(
        children: [
          Container(
            width: 44,
            height: 44,
            decoration: BoxDecoration(
              color: iconBg,
              borderRadius: BorderRadius.circular(10),
            ),
            child: Icon(icon, color: iconColor, size: 22),
          ),
          const SizedBox(width: 14),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(title, style: AzzenTypography.bodyBold),
                const SizedBox(height: 2),
                Text(
                  subtitle,
                  style: AzzenTypography.caption,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ],
            ),
          ),
          const Icon(CupertinoIcons.chevron_forward, size: 16, color: AzzenColors.textMuted),
        ],
      ),
    );
  }
}