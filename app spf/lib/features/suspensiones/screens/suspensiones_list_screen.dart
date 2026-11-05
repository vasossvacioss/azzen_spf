import 'package:flutter/cupertino.dart';
import '../../../core/models/suspension.dart';
import '../../../core/networking/api_client.dart';
import '../../../core/theme/azzen_colors.dart';
import '../../../core/theme/azzen_typography.dart';
import '../../../shared/widgets/azzen_badge.dart';
import '../../../shared/widgets/azzen_card.dart';
import 'suspension_detail_screen.dart';
class SuspensionesListScreen extends StatefulWidget {
  final String? initialFilterEstado;
  const SuspensionesListScreen({super.key, this.initialFilterEstado});
  @override
  State<SuspensionesListScreen> createState() => clase_2001();
}
class clase_2001 extends State<SuspensionesListScreen> {
  final List<SuspensionItem> v_4001 = [];
  bool v_4002 = false;
  String? v_4003;
  int v_4004 = 1;
  int v_4005 = 0;
  String v_4006 = 'todos';
  final v_4007 = TextEditingController();
  final List<String> v_4008 = ['todos', 'Vigente', 'Levantado'];
  @override
  void initState() {
    super.initState();
    if (widget.initialFilterEstado != null) {
      v_4006 = widget.initialFilterEstado!;
    }
    m_3001();
  }
  @override
  void dispose() {
    v_4007.dispose();
    super.dispose();
  }
  Future<void> m_3001({bool refresh = false}) async {
    if (refresh) {
      v_4004 = 1;
      v_4001.clear();
    }
    setState(() {
      v_4002 = true;
      v_4003 = null;
    });
    try {
      final query = <String, String>{
        'page': v_4004.toString(),
        'limit': '20',
      };
      if (v_4006 != 'todos') {
        query['estado'] = v_4006;
      }
      final q = v_4007.text.trim();
      if (q.isNotEmpty) {
        query['q'] = q;
      }
      final data = await ApiClient.get('/api/mobile/suspensiones', queryParams: query);
      final rawList = data['items'] as List? ?? [];
      final loaded = rawList.map((i) => SuspensionItem.fromJson(i)).toList();
      if (mounted) {
        setState(() {
          if (refresh) {
            v_4001.clear();
          }
          v_4001.addAll(loaded);
          v_4005 = data['total'] is int ? data['total'] : v_4001.length;
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
    return CupertinoPageScaffold(
      navigationBar: CupertinoNavigationBar(
        middle: const Text('Suspensiones Preventivas'),
        trailing: CupertinoButton(
          padding: EdgeInsets.zero,
          child: const Icon(CupertinoIcons.arrow_clockwise, size: 20),
          onPressed: () => m_3001(refresh: true),
        ),
      ),
      child: SafeArea(
        child: Column(
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 8, 16, 4),
              child: CupertinoSearchTextField(
                controller: v_4007,
                placeholder: 'Buscar por credencial, nombre o número de sumario...',
                onSubmitted: (_) => m_3001(refresh: true),
                style: AzzenTypography.body,
              ),
            ),
            SizedBox(
              height: 44,
              child: ListView.separated(
                padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
                scrollDirection: Axis.horizontal,
                itemCount: v_4008.length,
                separatorBuilder: (_, _) => const SizedBox(width: 8),
                itemBuilder: (ctx, i) {
                  final est = v_4008[i];
                  final isSelected = est == v_4006;
                  return GestureDetector(
                    onTap: () {
                      setState(() {
                        v_4006 = est;
                      });
                      m_3001(refresh: true);
                    },
                    child: Container(
                      padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 4),
                      decoration: BoxDecoration(
                        color: isSelected ? AzzenColors.navy : AzzenColors.surfaceSecondary,
                        borderRadius: BorderRadius.circular(20),
                        border: Border.all(color: isSelected ? AzzenColors.navy : AzzenColors.border),
                      ),
                      alignment: Alignment.center,
                      child: Text(
                        est == 'todos' ? 'Todas' : est,
                        style: TextStyle(
                          fontSize: 12,
                          fontWeight: isSelected ? FontWeight.w600 : FontWeight.w500,
                          color: isSelected ? CupertinoColors.white : AzzenColors.textPrimary,
                        ),
                      ),
                    ),
                  );
                },
              ),
            ),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Text('Medidas registradas: $v_4005', style: AzzenTypography.captionBold),
                  if (v_4002) const CupertinoActivityIndicator(radius: 8),
                ],
              ),
            ),
            const SizedBox(height: 4),
            Expanded(child: m_3002()),
          ],
        ),
      ),
    );
  }
  Widget m_3002() {
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
              const Icon(CupertinoIcons.wifi_exclamationmark, size: 40, color: AzzenColors.danger),
              const SizedBox(height: 8),
              Text(v_4003!, textAlign: TextAlign.center, style: AzzenTypography.caption),
              const SizedBox(height: 12),
              CupertinoButton(child: const Text('Reintentar'), onPressed: () => m_3001(refresh: true)),
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
            Icon(CupertinoIcons.shield_slash, size: 48, color: AzzenColors.textMuted.withValues(alpha: 0.5)),
            const SizedBox(height: 12),
            const Text('No se encontraron suspensiones preventivas.', style: TextStyle(color: AzzenColors.textMuted, fontSize: 13)),
          ],
        ),
      );
    }
    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: v_4001.length + (v_4001.length < v_4005 ? 1 : 0),
      itemBuilder: (ctx, i) {
        if (i == v_4001.length) {
          return Padding(
            padding: const EdgeInsets.symmetric(vertical: 16),
            child: CupertinoButton(
              child: const Text('Cargar más suspensiones'),
              onPressed: () {
                v_4004++;
                m_3001();
              },
            ),
          );
        }
        final s = v_4001[i];
        return AzzenCard(
          onTap: () {
            Navigator.push(
              context,
              CupertinoPageRoute(
                builder: (_) => SuspensionDetailScreen(suspensionId: s.id),
              ),
            );
          },
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: [
                  Row(
                    children: [
                      Container(
                        padding: const EdgeInsets.all(6),
                        decoration: BoxDecoration(
                          color: AzzenColors.dangerBackground,
                          borderRadius: BorderRadius.circular(6),
                        ),
                        child: const Icon(CupertinoIcons.exclamationmark_shield, size: 16, color: AzzenColors.danger),
                      ),
                      const SizedBox(width: 8),
                      Text('Cred. ${s.credencial}', style: AzzenTypography.bodyBold),
                    ],
                  ),
                  AzzenBadge.fromStatus(s.estado),
                ],
              ),
              const SizedBox(height: 6),
              Text(s.nombreCompleto, style: AzzenTypography.body),
              const SizedBox(height: 4),
              if (s.articulo419Incisos.isNotEmpty)
                Text('Artículo 419: ${s.articulo419Incisos}', style: AzzenTypography.captionBold.copyWith(color: AzzenColors.navy)),
              const SizedBox(height: 4),
              Row(
                children: [
                  if (s.numeroExpediente != null)
                    Text('Exp: ${s.numeroExpediente}', style: AzzenTypography.caption),
                  const Spacer(),
                  if (s.fechaInicio != null)
                    Text('Desde: ${s.fechaInicio}', style: AzzenTypography.caption.copyWith(color: AzzenColors.textMuted)),
                ],
              ),
            ],
          ),
        );
      },
    );
  }
}