import 'package:flutter/cupertino.dart';
import '../../core/theme/azzen_colors.dart';
import '../../core/theme/azzen_typography.dart';
class AzzenBadge extends StatelessWidget {
  final String text;
  final Color? backgroundColor;
  final Color? textColor;
  const AzzenBadge({
    super.key,
    required this.text,
    this.backgroundColor,
    this.textColor,
  });
  factory AzzenBadge.fromStatus(String status) {
    final s = status.toLowerCase();
    if (s.contains('vigente') || s.contains('activo') || s.contains('abierto')) {
      return AzzenBadge(
        text: status,
        backgroundColor: AzzenColors.successBackground,
        textColor: AzzenColors.successText,
      );
    } else if (s.contains('pendiente') || s.contains('proceso') || s.contains('revisión')) {
      return AzzenBadge(
        text: status,
        backgroundColor: AzzenColors.warningBackground,
        textColor: AzzenColors.warningText,
      );
    } else if (s.contains('suspendido') || s.contains('bloqueado') || s.contains('revocado') || s.contains('rechazado')) {
      return AzzenBadge(
        text: status,
        backgroundColor: AzzenColors.dangerBackground,
        textColor: AzzenColors.dangerText,
      );
    } else if (s.contains('levantado') || s.contains('archivado') || s.contains('finalizado')) {
      return AzzenBadge(
        text: status,
        backgroundColor: AzzenColors.navyLight,
        textColor: AzzenColors.navy,
      );
    } else {
      return AzzenBadge(
        text: status,
        backgroundColor: AzzenColors.infoBackground,
        textColor: AzzenColors.infoText,
      );
    }
  }
  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: backgroundColor ?? AzzenColors.surfaceSecondary,
        borderRadius: BorderRadius.circular(6),
      ),
      child: Text(
        text,
        style: AzzenTypography.badge.copyWith(
          color: textColor ?? AzzenColors.textSecondary,
        ),
      ),
    );
  }
}