import 'package:flutter/cupertino.dart';
import '../../../core/models/notificacion.dart';
import '../../../core/networking/api_client.dart';
import '../../../core/theme/azzen_colors.dart';
import '../../../core/theme/azzen_typography.dart';
import '../../../shared/widgets/azzen_card.dart';
import '../../expedientes/screens/expediente_detail_screen.dart';
class NotificacionesScreen extends StatefulWidget {
  const NotificacionesScreen({super.key});
  @override
  State<NotificacionesScreen> createState() => clase_2001();
}
class clase_2001 extends State<NotificacionesScreen> {
  final List<NotificacionModel> v_4001 = [];
  bool v_4002 = false;
  String? v_4003;
  @override
  void initState() {
    super.initState();
    m_3001();
  }
  Future<void> m_3001() async {
    setState(() {
      v_4002 = true;
      v_4003 = null;
    });
    try {
      final data = await ApiClient.get('/api/notificaciones');
      final notifsRaw = data['notificaciones'] as List? ?? (data is List ? data : []);
      final loaded = notifsRaw.map((n) => NotificacionModel.fromJson(n)).toList();
      if (mounted) {
        setState(() {
          v_4001.clear();
          v_4001.addAll(loaded);
          v_4002 = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          v_4003 = e.toString();
          v_4002 = false;
        });
      }
    }
  }
  Future<void> m_3002() async {
    try {
      await ApiClient.post('/api/notificaciones/marcar-todas');
      m_3001();
    } catch (_) {}
  }
  Future<void> m_3003(NotificacionModel n) async {
    if (!n.leido) {
      try {
        await ApiClient.post('/api/notificaciones/${n.id}/leer');
      } catch (_) {}
    }
    if (n.expedienteId != null && mounted) {
      Navigator.push(
        context,
        CupertinoPageRoute(
          builder: (_) => ExpedienteDetailScreen(expedienteId: n.expedienteId!),
        ),
      );
    }
  }
  @override
  Widget build(BuildContext context) {
    return CupertinoPageScaffold(
      navigationBar: CupertinoNavigationBar(
        middle: const Text('Notificaciones Institucionales'),
        trailing: CupertinoButton(
          padding: EdgeInsets.zero,
          onPressed: m_3002,
          child: const Text('Marcar todas', style: TextStyle(fontSize: 13)),
        ),
      ),
      child: SafeArea(
        child: m_3004(),
      ),
    );
  }
  Widget m_3004() {
    if (v_4002 && v_4001.isEmpty) {
      return const Center(child: CupertinoActivityIndicator(radius: 14));
    }
    if (v_4003 != null && v_4001.isEmpty) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(CupertinoIcons.bell_slash, size: 40, color: AzzenColors.danger),
              const SizedBox(height: 8),
              Text(v_4003!, textAlign: TextAlign.center, style: AzzenTypography.caption),
              const SizedBox(height: 12),
              CupertinoButton(onPressed: m_3001, child: const Text('Reintentar')),
            ],
          ),
        ),
      );
    }
    if (v_4001.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(CupertinoIcons.bell, size: 48, color: AzzenColors.textMuted.withValues(alpha: 0.5)),
            const SizedBox(height: 12),
            const Text('No tiene notificaciones pendientes.', style: TextStyle(color: AzzenColors.textMuted)),
          ],
        ),
      );
    }
    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: v_4001.length,
      itemBuilder: (ctx, i) {
        final n = v_4001[i];
        return AzzenCard(
          backgroundColor: n.leido ? AzzenColors.surfaceCard : AzzenColors.primaryLight.withValues(alpha: 0.4),
          onTap: () => m_3003(n),
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Container(
                margin: const EdgeInsets.only(top: 2),
                padding: const EdgeInsets.all(6),
                decoration: BoxDecoration(
                  color: n.leido ? AzzenColors.navyLight : AzzenColors.primary,
                  shape: BoxShape.circle,
                ),
                child: Icon(
                  CupertinoIcons.bell_fill,
                  size: 14,
                  color: n.leido ? AzzenColors.textMuted : CupertinoColors.white,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(n.titulo, style: AzzenTypography.bodyBold),
                    const SizedBox(height: 4),
                    Text(n.mensaje, style: AzzenTypography.body),
                    if (n.createdAt != null) ...[
                      const SizedBox(height: 6),
                      Text(n.createdAt!, style: AzzenTypography.caption.copyWith(color: AzzenColors.textMuted, fontSize: 11)),
                    ],
                  ],
                ),
              ),
              if (!n.leido)
                Container(
                  width: 8,
                  height: 8,
                  margin: const EdgeInsets.only(top: 6),
                  decoration: const BoxDecoration(
                    color: AzzenColors.primary,
                    shape: BoxShape.circle,
                  ),
                ),
            ],
          ),
        );
      },
    );
  }
}