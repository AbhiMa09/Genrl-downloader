import 'package:flutter/material.dart';
import 'package:provider/provider.dart';

import '../../services/state_provider.dart';
import 'tray_menu_popup.dart';
import 'tray_notification_toasts.dart';

class SystemTrayNotificationOverlay extends StatelessWidget {
  final VoidCallback onClose;

  const SystemTrayNotificationOverlay({super.key, required this.onClose});

  @override
  Widget build(BuildContext context) {
    final state = context.watch<StateProvider>();

    return Material(
      color: Colors.transparent,
      child: Stack(
        children: [
          // Dismissible background barrier
          Positioned.fill(
            child: GestureDetector(
              onTap: onClose,
              child: Container(color: Colors.black45),
            ),
          ),

          // Top right floating container matching prototype
          Positioned(
            top: 60,
            right: 24,
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 820),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  // LEFT: System Tray Popup Menu
                  SizedBox(
                    width: 330,
                    child: TrayMenuPopup(state: state, onClose: onClose),
                  ),
                  const SizedBox(width: 16),

                  // RIGHT: Desktop Notification Toasts (Stack of 3)
                  SizedBox(
                    width: 420,
                    child: TrayNotificationToasts(
                      state: state,
                      onClose: onClose,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }
}
