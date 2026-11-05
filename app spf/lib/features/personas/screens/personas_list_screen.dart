import 'package:flutter/cupertino.dart';
import '../../../core/models/persona.dart';
import '../../../core/networking/api_client.dart';
import '../../../core/theme/azzen_colors.dart';
import '../../../core/theme/azzen_typography.dart';
import '../../../shared/widgets/azzen_card.dart';
import 'persona_detail_screen.dart';
class PersonasListScreen extends StatefulWidget {
  const PersonasListScreen({super.key});
  @override
  State<PersonasListScreen> createState() => clase_2001();
}
class clase_2001 extends State<PersonasListScreen> {
  final List<PersonaItem> _personas = [];
  bool v_4001 = false;
  String? v_4002;
  final v_4003 = TextEditingController();
  @override
  void initState() {
    super.initState();
    m_3001();
  }
  @override
  void dispose() {
    v_4003.dispose();
    super.dispose();
  }
  Future<void> m_3001([String? query]) async {
    final q = query ?? v_4003.text.trim();
    setState(() {
      v_4001 = true;
      v_4002 = null;
    });
    try {
      final data = await ApiClient.get(
        '/api/personas/buscar',
        queryParams: {'q': q.isNotEmpty ? q : '900'},
      );
      final list = data['personas'] as List? ?? [];
      final loaded = list.map((p) => PersonaItem.fromJson(p)).toList();
      if (mounted) {
        setState(() {
          _personas.clear();
          _personas.addAll(loaded);
          v_4001 = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          v_4002 = e.toString();
          v_4001 = false;
        });
      }
    }
  }
  @override
  Widget build(BuildContext context) {
    return CupertinoPageScaffold(
      navigationBar: CupertinoNavigationBar(
        middle: const Text('Directorio de Personas'),
        trailing: CupertinoButton(
          padding: EdgeInsets.zero,
          child: const Icon(CupertinoIcons.arrow_clockwise, size: 20),
          onPressed: () => m_3001(),
        ),
      ),
      child: SafeArea(
        child: Column(
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
              child: CupertinoSearchTextField(
                controller: v_4003,
                placeholder: 'Buscar por credencial exacta, apellido o nombre...',
                onSubmitted: (q) => m_3001(q),
                style: AzzenTypography.body,
              ),
            ),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
              child: Row(
                children: [
                  Text('Resultados encontrados: ${_personas.length}', style: AzzenTypography.captionBold),
                ],
              ),
            ),
            const SizedBox(height: 4),
            Expanded(
              child: m_3002(),
            ),
          ],
        ),
      ),
    );
  }
  Widget m_3002() {
    if (v_4001 && _personas.isEmpty) {
      return const Center(child: CupertinoActivityIndicator(radius: 14));
    }
    if (v_4002 != null && _personas.isEmpty) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(CupertinoIcons.exclamationmark_circle, size: 40, color: AzzenColors.danger),
              const SizedBox(height: 8),
              Text(v_4002!, textAlign: TextAlign.center, style: AzzenTypography.caption),
              const SizedBox(height: 12),
              CupertinoButton(child: const Text('Reintentar'), onPressed: () => m_3001()),
            ],
          ),
        ),
      );
    }
    if (_personas.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(CupertinoIcons.person_badge_minus, size: 48, color: AzzenColors.textMuted.withValues(alpha: 0.5)),
            const SizedBox(height: 12),
            const Text('No se encontraron registros de personas.', style: TextStyle(color: AzzenColors.textMuted)),
          ],
        ),
      );
    }
    return ListView.builder(
      padding: const EdgeInsets.all(16),
      itemCount: _personas.length,
      itemBuilder: (ctx, i) {
        final p = _personas[i];
        return AzzenCard(
          onTap: () {
            Navigator.push(
              context,
              CupertinoPageRoute(
                builder: (_) => PersonaDetailScreen(credencial: p.credencial),
              ),
            );
          },
          child: Row(
            children: [
              Container(
                width: 44,
                height: 44,
                decoration: BoxDecoration(
                  color: AzzenColors.navyLight,
                  borderRadius: BorderRadius.circular(10),
                ),
                child: const Icon(CupertinoIcons.person_fill, color: AzzenColors.navy, size: 24),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(p.displayName, style: AzzenTypography.bodyBold),
                    const SizedBox(height: 2),
                    Text('Credencial: ${p.credencial}', style: AzzenTypography.captionBold.copyWith(color: AzzenColors.primary)),
                    if (p.escalafon != null || p.grado != null)
                      Text('${p.escalafon ?? ""} ${p.grado != null ? "· ${p.grado}" : ""}', style: AzzenTypography.caption),
                  ],
                ),
              ),
              const Icon(CupertinoIcons.chevron_forward, size: 16, color: AzzenColors.textMuted),
            ],
          ),
        );
      },
    );
  }
}