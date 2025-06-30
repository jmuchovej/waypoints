import 'package:flutter/material.dart';
import 'package:go_router/go_router.dart';
import '../ui/app_shell.dart';

class IndexPage extends StatelessWidget {
  final StatefulNavigationShell navigationShell;
  
  const IndexPage({super.key, required this.navigationShell});

  @override
  Widget build(BuildContext context) {
    return AppShell(navigationShell: navigationShell);
  }
}
