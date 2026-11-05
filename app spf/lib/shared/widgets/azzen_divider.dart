import 'package:flutter/cupertino.dart';
import '../../core/theme/azzen_colors.dart';
class AzzenDivider extends StatelessWidget {
  final Color? color;
  final double height;
  final EdgeInsetsGeometry margin;
  const AzzenDivider({
    super.key,
    this.color,
    this.height = 1.0,
    this.margin = const EdgeInsets.symmetric(vertical: 4),
  });
  @override
  Widget build(BuildContext context) {
    return Container(
      height: height,
      margin: margin,
      color: color ?? AzzenColors.borderLight,
    );
  }
}