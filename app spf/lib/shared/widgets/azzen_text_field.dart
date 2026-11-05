import 'package:flutter/cupertino.dart';
import '../../core/theme/azzen_colors.dart';
import '../../core/theme/azzen_typography.dart';
class AzzenTextField extends StatefulWidget {
  final String label;
  final String? placeholder;
  final TextEditingController controller;
  final bool isPassword;
  final TextInputType keyboardType;
  final IconData? prefixIcon;
  final String? errorText;
  final ValueChanged<String>? onChanged;
  final int maxLines;
  const AzzenTextField({
    super.key,
    required this.label,
    this.placeholder,
    required this.controller,
    this.isPassword = false,
    this.keyboardType = TextInputType.text,
    this.prefixIcon,
    this.errorText,
    this.onChanged,
    this.maxLines = 1,
  });
  @override
  State<AzzenTextField> createState() => clase_2001();
}
class clase_2001 extends State<AzzenTextField> {
  late bool v_4001;
  @override
  void initState() {
    super.initState();
    v_4001 = widget.isPassword;
  }
  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          widget.label,
          style: AzzenTypography.captionBold.copyWith(color: AzzenColors.navy),
        ),
        const SizedBox(height: 6),
        Container(
          decoration: BoxDecoration(
            color: AzzenColors.surfaceSecondary,
            borderRadius: BorderRadius.circular(10),
            border: Border.all(
              color: widget.errorText != null ? AzzenColors.danger : AzzenColors.border,
            ),
          ),
          padding: const EdgeInsets.symmetric(horizontal: 12),
          child: Row(
            children: [
              if (widget.prefixIcon != null) ...[
                Icon(widget.prefixIcon, size: 20, color: AzzenColors.textMuted),
                const SizedBox(width: 8),
              ],
              Expanded(
                child: CupertinoTextField(
                  controller: widget.controller,
                  obscureText: v_4001,
                  keyboardType: widget.keyboardType,
                  onChanged: widget.onChanged,
                  maxLines: widget.maxLines,
                  placeholder: widget.placeholder,
                  placeholderStyle: AzzenTypography.body.copyWith(color: AzzenColors.textMuted),
                  style: AzzenTypography.body,
                  decoration: null,
                  padding: const EdgeInsets.symmetric(vertical: 13),
                ),
              ),
              if (widget.isPassword)
                CupertinoButton(
                  padding: EdgeInsets.zero,
                  minimumSize: Size.zero,
                  onPressed: () {
                    setState(() {
                      v_4001 = !v_4001;
                    });
                  },
                  child: Icon(
                    v_4001 ? CupertinoIcons.eye : CupertinoIcons.eye_slash,
                    size: 20,
                    color: AzzenColors.textMuted,
                  ),
                ),
            ],
          ),
        ),
        if (widget.errorText != null) ...[
          const SizedBox(height: 4),
          Text(
            widget.errorText!,
            style: AzzenTypography.caption.copyWith(color: AzzenColors.danger),
          ),
        ],
      ],
    );
  }
}