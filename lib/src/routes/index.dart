import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import '../ui/shell/app_shell.dart';

@RoutePage(name: 'IndexRoute')
class IndexPage extends StatelessWidget {
  const IndexPage({super.key});

  @override
  Widget build(BuildContext context) {
    return const AppShell();
  }
}
