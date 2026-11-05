import 'package:flutter/cupertino.dart';
import '../../../core/models/suspension.dart';
import '../../../core/networking/api_client.dart';
import '../../../core/theme/azzen_colors.dart';
import '../../../core/theme/azzen_typography.dart';
import '../../../shared/widgets/azzen_badge.dart';
import '../../../shared/widgets/azzen_card.dart';
import '../../../shared/widgets/azzen_divider.dart';
import '../../expedientes/screens/expediente_detail_screen.dart';
import '../../personas/screens/persona_detail_screen.dart';
class SuspensionDetailScreen extends StatefulWidget {
  final int suspensionId;
  const SuspensionDetailScreen({super.key, required this.suspensionId});
  @override
  State<SuspensionDetailScreen> createState() => clase_2001();
}
class clase_2001 extends State<SuspensionDetailScreen> {
  SuspensionItem? v_4001;
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
      final data = await ApiClient.get('/api/mobile/suspensiones/${widget.suspensionId}');
      final raw = data['suspension'] as Map<String, dynamic>? ?? {};
      if (mounted) {
        setState(() {
          v_4001 = SuspensionItem.fromJson(raw);
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
        navigationBar: CupertinoNavigationBar(middle: Text('Cargando suspensión...')),
        child: Center(child: CupertinoActivityIndicator(radius: 14)),
      );
    }
    if (v_4003 != null || v_4001 == null) {
      return CupertinoPageScaffold(
        navigationBar: const CupertinoNavigationBar(middle: Text('Medida Cautelar')),
        child: Center(
          child: Padding(
            padding: const EdgeInsets.all(24),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(CupertinoIcons.exclamationmark_triangle, size: 40, color: AzzenColors.danger),
                const SizedBox(height: 8),
                Text(v_4003 ?? 'No encontrada', textAlign: TextAlign.center, style: AzzenTypography.caption),
                const SizedBox(height: 12),
                CupertinoButton(onPressed: m_3001, child: const Text('Reintentar')),
              ],
            ),
          ),
        ),
      );
    }
    final s = v_4001!;
    return CupertinoPageScaffold(
      navigationBar: CupertinoNavigationBar(
        middle: Text('Suspensión #${s.id}'),
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
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      Text('Medida Preventiva #${s.id}', style: AzzenTypography.header3),
                      AzzenBadge.fromStatus(s.estado),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'Régimen Disciplinario SPF · Artículo 419 del Reglamento',
                    style: AzzenTypography.caption.copyWith(color: AzzenColors.primary),
                  ),
                ],
              ),
            ),
            Row(
              children: [
                const Icon(CupertinoIcons.person_crop_circle_fill, size: 18, color: AzzenColors.navy),
                const SizedBox(width: 6),
                Text('AGENTE IMPUTADO', style: AzzenTypography.captionBold.copyWith(color: AzzenColors.navy)),
              ],
            ),
            const SizedBox(height: 6),
            AzzenCard(
              onTap: () {
                Navigator.push(
                  context,
                  CupertinoPageRoute(
                    builder: (_) => PersonaDetailScreen(credencial: s.credencial),
                  ),
                );
              },
              child: Row(
                children: [
                  Container(
                    padding: const EdgeInsets.all(8),
                    decoration: BoxDecoration(
                      color: AzzenColors.navyLight,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: const Icon(CupertinoIcons.person_fill, size: 22, color: AzzenColors.navy),
                  ),
                  const SizedBox(width: 12),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(s.nombreCompleto, style: AzzenTypography.bodyBold),
                        const SizedBox(height: 2),
                        Text('Credencial: ${s.credencial}', style: AzzenTypography.captionBold.copyWith(color: AzzenColors.primary)),
                      ],
                    ),
                  ),
                  const Icon(CupertinoIcons.chevron_forward, size: 16, color: AzzenColors.textMuted),
                ],
              ),
            ),
            const SizedBox(height: 12),
            if (s.expedienteId != null) ...[
              Row(
                children: [
                  const Icon(CupertinoIcons.doc_text_fill, size: 18, color: AzzenColors.primary),
                  const SizedBox(width: 6),
                  Text('EXPEDIENTE DE ORIGEN', style: AzzenTypography.captionBold.copyWith(color: AzzenColors.navy)),
                ],
              ),
              const SizedBox(height: 6),
              AzzenCard(
                onTap: () {
                  Navigator.push(
                    context,
                    CupertinoPageRoute(
                      builder: (_) => ExpedienteDetailScreen(expedienteId: s.expedienteId!),
                    ),
                  );
                },
                child: Row(
                  children: [
                    Expanded(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text('Exp. ${s.numeroExpediente ?? "S/N"}', style: AzzenTypography.bodyBold),
                          if (s.reparticion != null)
                            Text('Repartición: ${s.reparticion}', style: AzzenTypography.caption),
                        ],
                      ),
                    ),
                    const Icon(CupertinoIcons.chevron_forward, size: 16, color: AzzenColors.textMuted),
                  ],
                ),
              ),
              const SizedBox(height: 12),
            ],
            Row(
              children: [
                const Icon(CupertinoIcons.shield_fill, size: 18, color: AzzenColors.navy),
                const SizedBox(width: 6),
                Text('ENCUADRE NORMATIVO Y VIGENCIA', style: AzzenTypography.captionBold.copyWith(color: AzzenColors.navy)),
              ],
            ),
            const SizedBox(height: 6),
            AzzenCard(
              child: Column(
                children: [
                  m_3002('Incisos Art. 419', s.articulo419Incisos.isNotEmpty ? s.articulo419Incisos : 'No especificado'),
                  const AzzenDivider(),
                  m_3002('Fecha de inicio', s.fechaInicio ?? '—'),
                  const AzzenDivider(),
                  m_3002('Fecha de cese', s.fechaFin ?? 'Medida activa'),
                  if (s.motivoFin != null && s.motivoFin!.isNotEmpty) ...[
                    const AzzenDivider(),
                    m_3002('Motivo de cese', s.motivoFin!),
                  ],
                  if (s.fundamentoFin != null && s.fundamentoFin!.isNotEmpty) ...[
                    const AzzenDivider(),
                    m_3002('Fundamento cese', s.fundamentoFin!),
                  ],
                  if (s.observaciones != null && s.observaciones!.isNotEmpty) ...[
                    const AzzenDivider(),
                    m_3002('Observaciones', s.observaciones!),
                  ],
                ],
              ),
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
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(width: 130, child: Text(label, style: AzzenTypography.captionBold)),
          Expanded(child: Text(value, style: AzzenTypography.body)),
        ],
      ),
    );
  }
}