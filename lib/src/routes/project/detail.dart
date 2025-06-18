import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';

import '../../router.dart';

@RoutePage(name: 'ProjectDetailRoute')
class ProjectDetailPage extends StatelessWidget {
  const ProjectDetailPage({super.key, @PathParam('uuid') required this.uuid});

  final String uuid;

  @override
  Widget build(BuildContext context) {
    // Redirect to the universal thing route for consistent editing experience
    WidgetsBinding.instance.addPostFrameCallback((_) {
      context.router.replaceAll([ThingRoute(uuid: uuid)]);
    });

    return const Scaffold(body: Center(child: CircularProgressIndicator()));
  }
}
