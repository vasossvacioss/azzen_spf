import 'package:flutter/cupertino.dart';
import '../../core/theme/azzen_colors.dart';
import '../../core/theme/azzen_typography.dart';
enum AzzenButtonVariant { primary, secondary, danger, outline }
class AzzenButton extends StatelessWidget {
  final String label;
  final VoidCallback? onPressed;
  final bool isLoading;
  final IconData? icon;
  final AzzenButtonVariant variant;
  final double height;
  const AzzenButton({
    super.key,
    required this.label,
    required this.onPressed,
    this.isLoading = false,
    this.icon,
    this.variant = AzzenButtonVariant.primary,
    this.height = 48,
  });
  @override
  Widget build(BuildContext context) {
    Color bgColor;
    Color textColor;
    Border? border;
    switch (variant) {
      case AzzenButtonVariant.primary:
        bgColor = AzzenColors.primary;
        textColor = CupertinoColors.white;
        break;
      case AzzenButtonVariant.secondary:
        bgColor = AzzenColors.surfaceSecondary;
        textColor = AzzenColors.navy;
        border = Border.all(color: AzzenColors.border);
        break;
      case AzzenButtonVariant.danger:
        bgColor = AzzenColors.danger;
        textColor = CupertinoColors.white;
        break;
      case AzzenButtonVariant.outline:
        bgColor = CupertinoColors.transparent;
        textColor = AzzenColors.primary;
        border = Border.all(color: AzzenColors.primary);
        break;
    }
    if (onPressed == null || isLoading) {
      bgColor = bgColor.withValues(alpha: 0.6);
      textColor = textColor.withValues(alpha: 0.7);
    }
    return GestureDetector(
      onTap: (isLoading || onPressed == null) ? null : onPressed,
      child: Container(
        height: height,
        decoration: BoxDecoration(
          color: bgColor,
          borderRadius: BorderRadius.circular(10),
          border: border,
        ),
        alignment: Alignment.center,
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: isLoading
            ? const CupertinoActivityIndicator(color: CupertinoColors.white)
            : Row(
                mainAxisSize: MainAxisSize.min,
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  if (icon != null) ...[
                    Icon(icon, size: 18, color: textColor),
                    const SizedBox(width: 8),
                  ],
                  Text(
                    label,
                    style: AzzenTypography.bodyBold.copyWith(color: textColor),
                  ),
                ],
              ),
      ),
    );
  }
}