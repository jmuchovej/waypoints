import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../router.dart';

class ThingNotFoundScreen extends ConsumerWidget {
  const ThingNotFoundScreen({super.key, required this.uuid, this.type});

  final String uuid;
  final String? type;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Scaffold(
      appBar: AppBar(title: const Text('Thing Not Found')),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            const Icon(Icons.search_off, size: 64, color: Colors.grey),
            const SizedBox(height: 16),
            const Text(
              "Thing not found",
              style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 8),
            Text('UUID: $uuid', style: const TextStyle(color: Colors.grey)),
            if (type != null) ...[
              const SizedBox(height: 4),
              Text('Type: $type', style: const TextStyle(color: Colors.grey)),
            ],
            const SizedBox(height: 16),
            ElevatedButton(
              onPressed: () => context.gotoHome(),
              child: const Text('Return home'),
            ),
          ],
        ),
      ),
    );
  }
}
