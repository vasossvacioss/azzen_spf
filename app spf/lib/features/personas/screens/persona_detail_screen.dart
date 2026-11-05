import 'package:flutter/cupertino.dart';
import '../../../core/models/persona.dart';
import '../../../core/networking/api_client.dart';
import '../../../core/theme/azzen_colors.dart';
import '../../../core/theme/azzen_typography.dart';
import '../../../shared/widgets/azzen_badge.dart';
import '../../../shared/widgets/azzen_card.dart';
import '../../../shared/widgets/azzen_divider.dart';
import '../../expedientes/screens/expediente_detail_screen.dart';
import '../../suspensiones/screens/suspension_detail_screen.dart';
class PersonaDetailScreen extends StatefulWidget {
  final String credencial;
  const PersonaDetailScreen({super.key, required this.credencial});
  @override
  State<PersonaDetailScreen> createState() => clase_2001();
}
class clase_2001 extends State<PersonaDetailScreen> {
  PersonaDetalle? v_4001;
  bool v_4002 = true;
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
      final data = await ApiClient.get('/api/mobile/personas/${widget.credencial}');
      if (mounted) {
        setState(() {
          v_4001 = PersonaDetalle.fromJson(data);
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
  @override
  Widget build(BuildContext context) {
    if (v_4002) {
      return const CupertinoPageScaffold(
        navigationBar: CupertinoNavigationBar(middle: Text('Cargando ficha...')),
        child: Center(child: CupertinoActivityIndicator(radius: 14)),
      );
    }
    if (v_4003 != null || v_4001 == null) {
      return CupertinoPageScaffold(
        navigationBar: const CupertinoNavigationBar(middle: Text('Ficha de Agente')),
        child: Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(CupertinoIcons.exclamationmark_triangle, size: 40, color: AzzenColors.danger),
                const SizedBox(height: 8),
                Text(v_4003 ?? 'No encontrado', textAlign: TextAlign.center, style: AzzenTypography.caption),
                const SizedBox(height: 12),
                CupertinoButton(onPressed: m_3001, child: const Text('Reintentar')),
              ],
            ),
          ),
        ),
      );
    }
    final p = v_4001!.persona;
    final exps = v_4001!.expedientes;
    final susps = v_4001!.suspensiones;
    return CupertinoPageScaffold(
      navigationBar: CupertinoNavigationBar(
        middle: Text('Credencial ${p.credencial}'),
      ),
      child: SafeArea(
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            AzzenCard(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Container(
                        width: 52,
                        height: 52,
                        decoration: BoxDecoration(
                          color: AzzenColors.primaryLight,
                          borderRadius: BorderRadius.circular(12),
                        ),
                        child: const Icon(CupertinoIcons.person_solid, size: 28, color: AzzenColors.primary),
                      ),
                      const SizedBox(width: 14),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(p.displayName, style: AzzenTypography.header3),
                            const SizedBox(height: 2),
                            Text(
                              'Credencial Oficial: ${p.credencial}',
                              style: AzzenTypography.captionBold.copyWith(color: AzzenColors.primary),
                            ),
                          ],
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 16),
                  const AzzenDivider(),
                  const SizedBox(height: 8),
                  m_3002('Escalafón', p.escalafon ?? 'Seguridad'),
                  m_3002('Grado', p.grado ?? 'Sin grado especificado'),
                  if (p.origen != null)
                    m_3002('Registro origen', p.origen!),
                ],
              ),
            ),
            const SizedBox(height: 12),
            Row(
              children: [
                const Icon(CupertinoIcons.exclamationmark_shield_fill, size: 18, color: AzzenColors.danger),
                const SizedBox(width: 6),
                Text(
                  'SUSPENSIONES PREVENTIVAS (${susps.length})',
                  style: AzzenTypography.captionBold.copyWith(color: AzzenColors.navy),
                ),
              ],
            ),
            const SizedBox(height: 8),
            if (susps.isEmpty)
              const AzzenCard(
                child: Text('El agente no registra suspensiones preventivas asociadas.', style: TextStyle(color: AzzenColors.textMuted)),
              )
            else
              ...susps.map((s) => AzzenCard(
                onTap: () {
                  Navigator.push(
                    context,
                    CupertinoPageRoute(
                      builder: (_) => SuspensionDetailScreen(suspensionId: s.id),
                    ),
                  );
                },
                child: Row(
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
                            children: [
                              Text('Suspensión #${s.id}', style: AzzenTypography.bodyBold),
                              const SizedBox(width: 8),
                              AzzenBadge.fromStatus(s.estado),
                            ],
                          ),
                          const SizedBox(height: 4),
                          if (s.articulo419Incisos.isNotEmpty)
                            Text('Artículo 419: ${s.articulo419Incisos}', style: AzzenTypography.caption),
                          if (s.fechaInicio != null)
                            Text('Inicio: ${s.fechaInicio}${s.fechaFin != null ? " · Cese: ${s.fechaFin}" : ""}', style: AzzenTypography.caption),
                        ],
                      ),
                    ),
                    const Icon(CupertinoIcons.chevron_forward, size: 16, color: AzzenColors.textMuted),
                  ],
                ),
              )),
            const SizedBox(height: 16),
            Row(
              children: [
                const Icon(CupertinoIcons.doc_text_fill, size: 18, color: AzzenColors.primary),
                const SizedBox(width: 6),
                Text(
                  'EXPEDIENTES Y SUMARIOS VINCULADOS (${exps.length})',
                  style: AzzenTypography.captionBold.copyWith(color: AzzenColors.navy),
                ),
              ],
            ),
            const SizedBox(height: 8),
            if (exps.isEmpty)
              const AzzenCard(
                child: Text('No registra sumarios o expedientes vinculados directamente.', style: TextStyle(color: AzzenColors.textMuted)),
              )
            else
              ...exps.map((e) => AzzenCard(
                onTap: () {
                  Navigator.push(
                    context,
                    CupertinoPageRoute(
                      builder: (_) => ExpedienteDetailScreen(expedienteId: e.id),
                    ),
                  );
                },
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Text('Exp. ${e.numeroExpediente}', style: AzzenTypography.bodyBold),
                        AzzenBadge.fromStatus(e.estado),
                      ],
                    ),
                    const SizedBox(height: 4),
                    Text(e.motivo.isNotEmpty ? e.motivo : 'Sin carátula', style: AzzenTypography.body),
                    const SizedBox(height: 6),
                    Row(
                      children: [
                        Text('Rol: ${e.rol}', style: AzzenTypography.captionBold.copyWith(color: AzzenColors.navy)),
                        const Spacer(),
                        Text(e.reparticion, style: AzzenTypography.caption),
                      ],
                    ),
                  ],
                ),
              )),
          ],
        ),
      ),
    );
  }
  Widget m_3002(String label, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Row(
        children: [
          SizedBox(width: 130, child: Text(label, style: AzzenTypography.captionBold)),
          Expanded(child: Text(value, style: AzzenTypography.body)),
        ],
      ),
    );
  }
}