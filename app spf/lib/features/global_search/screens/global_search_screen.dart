import 'dart:async';
import 'package:flutter/cupertino.dart';
import '../../../core/models/global_search_result.dart';
import '../../../core/networking/api_client.dart';
import '../../../core/theme/azzen_colors.dart';
import '../../../core/theme/azzen_typography.dart';
import '../../../shared/widgets/azzen_badge.dart';
import '../../../shared/widgets/azzen_card.dart';
import '../../expedientes/screens/expediente_detail_screen.dart';
import '../../personas/screens/persona_detail_screen.dart';
import '../../suspensiones/screens/suspension_detail_screen.dart';
class GlobalSearchScreen extends StatefulWidget {
  final String? initialQuery;
  const GlobalSearchScreen({super.key, this.initialQuery});
  @override
  State<GlobalSearchScreen> createState() => clase_2001();
}
class clase_2001 extends State<GlobalSearchScreen> {
  final v_4001 = TextEditingController();
  Timer? v_4002;
  bool v_4003 = false;
  String? v_4004;
  GlobalSearchResult v_4005 = GlobalSearchResult();
  int v_4006 = 0;
  @override
  void initState() {
    super.initState();
    if (widget.initialQuery != null && widget.initialQuery!.isNotEmpty) {
      v_4001.text = widget.initialQuery!;
      m_3001(widget.initialQuery!);
    }
  }
  @override
  void dispose() {
    v_4002?.cancel();
    v_4001.dispose();
    super.dispose();
  }
  void m_3002(String query) {
    v_4002?.cancel();
    v_4002 = Timer(const Duration(milliseconds: 350), () {
      if (query.trim().length >= 2) {
        m_3001(query.trim());
      } else {
        setState(() {
          v_4005 = GlobalSearchResult();
          v_4004 = null;
        });
      }
    });
  }
  Future<void> m_3001(String query) async {
    setState(() {
      v_4003 = true;
      v_4004 = null;
    });
    try {
      final data = await ApiClient.get('/api/global-search', queryParams: {'q': query});
      if (mounted) {
        setState(() {
          v_4005 = GlobalSearchResult.fromJson(data);
          v_4003 = false;
        });
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          v_4004 = e.toString();
          v_4003 = false;
        });
      }
    }
  }
  @override
  Widget build(BuildContext context) {
    return CupertinoPageScaffold(
      navigationBar: const CupertinoNavigationBar(
        middle: Text('Búsqueda Transversal'),
      ),
      child: SafeArea(
        child: Column(
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 12, 16, 8),
              child: CupertinoSearchTextField(
                controller: v_4001,
                placeholder: 'Buscar credencial, apellido, número de sumario...',
                onChanged: m_3002,
                style: AzzenTypography.body,
              ),
            ),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
              child: SizedBox(
                width: double.infinity,
                child: CupertinoSlidingSegmentedControl<int>(
                  groupValue: v_4006,
                  children: const {
                    0: Padding(padding: EdgeInsets.symmetric(horizontal: 8), child: Text('Todos')),
                    1: Padding(padding: EdgeInsets.symmetric(horizontal: 8), child: Text('Expedientes')),
                    2: Padding(padding: EdgeInsets.symmetric(horizontal: 8), child: Text('Personas')),
                    3: Padding(padding: EdgeInsets.symmetric(horizontal: 8), child: Text('Suspensiones')),
                  },
                  onValueChanged: (val) {
                    if (val != null) {
                      setState(() {
                        v_4006 = val;
                      });
                    }
                  },
                ),
              ),
            ),
            const SizedBox(height: 8),
            Expanded(
              child: m_3003(),
            ),
          ],
        ),
      ),
    );
  }
  Widget m_3003() {
    if (v_4003) {
      return const Center(child: CupertinoActivityIndicator(radius: 14));
    }
    if (v_4004 != null) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              const Icon(CupertinoIcons.exclamationmark_circle, size: 40, color: AzzenColors.danger),
              const SizedBox(height: 8),
              Text(v_4004!, textAlign: TextAlign.center, style: AzzenTypography.caption),
            ],
          ),
        ),
      );
    }
    final query = v_4001.text.trim();
    if (query.length < 2) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(CupertinoIcons.search, size: 48, color: AzzenColors.textMuted.withValues(alpha: 0.5)),
            const SizedBox(height: 12),
            const Text(
              'Ingrese al menos 2 caracteres para buscar\npersonas, sumarios o suspensiones preventivas.',
              textAlign: TextAlign.center,
              style: TextStyle(color: AzzenColors.textMuted, fontSize: 13),
            ),
          ],
        ),
      );
    }
    if (v_4005.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(CupertinoIcons.tray, size: 48, color: AzzenColors.textMuted.withValues(alpha: 0.5)),
            const SizedBox(height: 12),
            Text(
              'No se encontraron coincidencias para "$query".',
              style: const TextStyle(color: AzzenColors.textMuted, fontSize: 13),
            ),
          ],
        ),
      );
    }
    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        if (v_4006 == 0 || v_4006 == 2) ...[
          if (v_4005.agentes.isNotEmpty) ...[
            m_3004('PERSONAS Y AGENTES (${v_4005.agentes.length})', CupertinoIcons.person_2_fill),
            ...v_4005.agentes.map(m_3005),
            const SizedBox(height: 16),
          ],
        ],
        if (v_4006 == 0 || v_4006 == 1) ...[
          if (v_4005.expedientes.isNotEmpty) ...[
            m_3004('EXPEDIENTES Y SUMARIOS (${v_4005.expedientes.length})', CupertinoIcons.doc_text_fill),
            ...v_4005.expedientes.map(m_3006),
            const SizedBox(height: 16),
          ],
        ],
        if (v_4006 == 0 || v_4006 == 3) ...[
          if (v_4005.suspensiones.isNotEmpty) ...[
            m_3004('SUSPENSIONES PREVENTIVAS (${v_4005.suspensiones.length})', CupertinoIcons.exclamationmark_shield_fill),
            ...v_4005.suspensiones.map(m_3007),
          ],
        ],
      ],
    );
  }
  Widget m_3004(String title, IconData icon) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8, top: 4),
      child: Row(
        children: [
          Icon(icon, size: 16, color: AzzenColors.primary),
          const SizedBox(width: 6),
          Text(
            title,
            style: const TextStyle(
              fontSize: 12,
              fontWeight: FontWeight.w700,
              color: AzzenColors.navy,
              letterSpacing: 0.5,
            ),
          ),
        ],
      ),
    );
  }
  Widget m_3005(SearchAgenteResult a) {
    return AzzenCard(
      onTap: () {
        Navigator.push(
          context,
          CupertinoPageRoute(
            builder: (_) => PersonaDetailScreen(credencial: a.credencial),
          ),
        );
      },
      child: Row(
        children: [
          Container(
            width: 40,
            height: 40,
            decoration: BoxDecoration(
              color: AzzenColors.navyLight,
              borderRadius: BorderRadius.circular(8),
            ),
            child: const Icon(CupertinoIcons.person_fill, color: AzzenColors.navy, size: 22),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(a.nombre, style: AzzenTypography.bodyBold),
                const SizedBox(height: 2),
                Text('Credencial: ${a.credencial} · Origen: ${a.origen}', style: AzzenTypography.caption),
              ],
            ),
          ),
          const Icon(CupertinoIcons.chevron_forward, size: 16, color: AzzenColors.textMuted),
        ],
      ),
    );
  }
  Widget m_3006(SearchExpedienteResult e) {
    return AzzenCard(
      onTap: () {
        Navigator.push(
          context,
          CupertinoPageRoute(
            builder: (_) => ExpedienteDetailScreen(expedienteId: e.id),
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
                    Text(e.numero, style: AzzenTypography.bodyBold),
                    const SizedBox(width: 8),
                    AzzenBadge.fromStatus(e.estado),
                  ],
                ),
                const SizedBox(height: 4),
                Text(
                  e.caratula,
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
  Widget m_3007(SearchSuspensionResult s) {
    return AzzenCard(
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
                    Text('Cred. ${s.credencial}', style: AzzenTypography.bodyBold),
                    const SizedBox(width: 8),
                    AzzenBadge.fromStatus(s.estado),
                  ],
                ),
                const SizedBox(height: 4),
                Text(s.nombre, style: AzzenTypography.body),
                if (s.articulo.isNotEmpty) ...[
                  const SizedBox(height: 2),
                  Text('Art. 419: ${s.articulo}', style: AzzenTypography.caption),
                ],
              ],
            ),
          ),
          const Icon(CupertinoIcons.chevron_forward, size: 16, color: AzzenColors.textMuted),
        ],
      ),
    );
  }
}