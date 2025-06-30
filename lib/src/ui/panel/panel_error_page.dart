import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';

/// Error page for panel system failures with recovery options
class PanelErrorPage extends StatelessWidget {
  const PanelErrorPage({
    super.key,
    required this.error,
    required this.originalPath,
  });

  final String error;
  final String originalPath;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    
    // Extract panels from the original path for recovery options
    final panels = _extractPanelsFromPath(originalPath);
    
    return Scaffold(
      appBar: AppBar(
        title: const Text('Panel System Error'),
        backgroundColor: Colors.red.withOpacity(0.1),
        elevation: 0,
      ),
      body: Padding(
        padding: const EdgeInsets.all(24.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Error header
            Row(
              children: [
                Icon(
                  Icons.error_outline,
                  color: Colors.red.shade600,
                  size: 32,
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    'Panel Navigation Error',
                    style: theme.textTheme.headlineSmall?.copyWith(
                      color: Colors.red.shade700,
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                ),
              ],
            ),
            
            const SizedBox(height: 16),
            
            // Error details
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: Colors.red.withOpacity(0.1),
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.red.withOpacity(0.3)),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Text(
                    'What happened:',
                    style: theme.textTheme.titleSmall?.copyWith(
                      fontWeight: FontWeight.bold,
                    ),
                  ),
                  const SizedBox(height: 8),
                  Text(
                    error,
                    style: theme.textTheme.bodyMedium,
                  ),
                  const SizedBox(height: 12),
                  Text(
                    'Original path: $originalPath',
                    style: theme.textTheme.bodySmall?.copyWith(
                      fontFamily: 'monospace',
                      color: theme.colorScheme.onSurfaceVariant,
                    ),
                  ),
                ],
              ),
            ),
            
            const SizedBox(height: 24),
            
            // Recovery options
            Text(
              'Recovery Options',
              style: theme.textTheme.titleLarge?.copyWith(
                fontWeight: FontWeight.bold,
              ),
            ),
            
            const SizedBox(height: 16),
            
            // Recovery buttons
            Wrap(
              spacing: 12,
              runSpacing: 12,
              children: [
                // Go back to demo home
                ElevatedButton.icon(
                  onPressed: () => context.go('/demo'),
                  icon: const Icon(Icons.home_outlined),
                  label: const Text('Return to Demo'),
                  style: ElevatedButton.styleFrom(
                    backgroundColor: theme.colorScheme.primary,
                    foregroundColor: Colors.white,
                  ),
                ),
                
                // Go to first panel if we have panels
                if (panels.isNotEmpty)
                  ElevatedButton.icon(
                    onPressed: () => context.go('/demo/thing/${panels.first}'),
                    icon: const Icon(Icons.view_column_outlined),
                    label: const Text('Go to First Panel'),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: Colors.blue,
                      foregroundColor: Colors.white,
                    ),
                  ),
                
                // Go to safe depth (max 5 panels)
                if (panels.length > 5)
                  ElevatedButton.icon(
                    onPressed: () {
                      final safePanels = panels.take(5).toList();
                      final safePath = '/demo/thing/${safePanels.join('/thing/')}';
                      context.go(safePath);
                    },
                    icon: const Icon(Icons.security),
                    label: const Text('Go to Safe Depth (5 panels)'),
                    style: ElevatedButton.styleFrom(
                      backgroundColor: Colors.green,
                      foregroundColor: Colors.white,
                    ),
                  ),
              ],
            ),
            
            const SizedBox(height: 24),
            
            // Information section
            Container(
              padding: const EdgeInsets.all(16),
              decoration: BoxDecoration(
                color: Colors.blue.withOpacity(0.1),
                borderRadius: BorderRadius.circular(8),
                border: Border.all(color: Colors.blue.withOpacity(0.3)),
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Icon(
                        Icons.info_outline,
                        color: Colors.blue.shade600,
                        size: 20,
                      ),
                      const SizedBox(width: 8),
                      Text(
                        'About Panel Limits',
                        style: theme.textTheme.titleSmall?.copyWith(
                          fontWeight: FontWeight.bold,
                          color: Colors.blue.shade700,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  Text(
                    'While the panel system supports unlimited nesting, very deep panel hierarchies can impact performance. We recommend keeping the depth under 20 panels for optimal user experience.',
                    style: theme.textTheme.bodySmall,
                  ),
                  const SizedBox(height: 8),
                  if (panels.length > 20)
                    Text(
                      'Your current depth: ${panels.length} panels',
                      style: theme.textTheme.bodySmall?.copyWith(
                        fontWeight: FontWeight.bold,
                        color: Colors.orange.shade700,
                      ),
                    ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }

  /// Extract panel UUIDs from path for recovery options
  List<String> _extractPanelsFromPath(String path) {
    final regex = RegExp(r'/thing/([^/]+)');
    final matches = regex.allMatches(path);
    return matches.map((match) => match.group(1)!).toList();
  }
}