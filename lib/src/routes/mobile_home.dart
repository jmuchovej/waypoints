import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

/// Mobile home page that displays the main navigation options
/// This replaces the sidebar on mobile, showing Tasks, Projects, and DevTools
class MobileHomeScreen extends ConsumerWidget {
  const MobileHomeScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    
    return Scaffold(
      backgroundColor: theme.colorScheme.surfaceContainer,
      body: SafeArea(
        child: Column(
          children: [
            // App header
            Container(
              height: 120,
              padding: const EdgeInsets.all(24),
              child: Row(
                children: [
                  Icon(
                    Icons.music_note_rounded,
                    size: 48,
                    color: theme.colorScheme.primary,
                  ),
                  const SizedBox(width: 16),
                  Expanded(
                    child: Text(
                      'Tempo',
                      style: theme.textTheme.headlineMedium?.copyWith(
                        fontWeight: FontWeight.bold,
                        color: theme.colorScheme.primary,
                      ),
                    ),
                  ),
                ],
              ),
            ),

            // Main navigation options
            Expanded(
              child: ListView(
                padding: const EdgeInsets.symmetric(horizontal: 24),
                children: [
                  _MobileNavCard(
                    icon: Icons.format_list_bulleted_rounded,
                    title: 'Tasks',
                    subtitle: 'Manage your tasks and todos',
                    color: theme.colorScheme.primary,
                    onTap: () => context.go('/tasks'),
                  ),
                  const SizedBox(height: 16),
                  _MobileNavCard(
                    icon: Icons.folder_rounded,
                    title: 'Projects',
                    subtitle: 'Organize work into projects',
                    color: theme.colorScheme.secondary,
                    onTap: () => context.go('/projects'),
                  ),
                  const SizedBox(height: 24),
                  
                  // Development section header
                  Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 16,
                      vertical: 8,
                    ),
                    child: Text(
                      'Development',
                      style: theme.textTheme.labelLarge?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ),
                  _MobileNavCard(
                    icon: Icons.developer_board,
                    title: 'DevTools',
                    subtitle: 'Development tools and debugging',
                    color: theme.colorScheme.tertiary,
                    onTap: () => context.go('/devtools'),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Individual navigation card for mobile home
class _MobileNavCard extends StatelessWidget {
  const _MobileNavCard({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.color,
    required this.onTap,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final Color color;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Card(
      elevation: 2,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(12),
        child: Padding(
          padding: const EdgeInsets.all(20),
          child: Row(
            children: [
              Container(
                width: 56,
                height: 56,
                decoration: BoxDecoration(
                  color: color.withOpacity(0.1),
                  borderRadius: BorderRadius.circular(12),
                ),
                child: Icon(
                  icon,
                  size: 32,
                  color: color,
                ),
              ),
              const SizedBox(width: 16),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      title,
                      style: theme.textTheme.titleLarge?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      subtitle,
                      style: theme.textTheme.bodyMedium?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              Icon(
                Icons.arrow_forward_ios,
                size: 16,
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ],
          ),
        ),
      ),
    );
  }
}